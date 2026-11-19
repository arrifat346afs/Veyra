//! Clipboard decoding and collision-safe filesystem operations.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub fn clipboard_paths(text: &str) -> Vec<PathBuf> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('#') {
                return None;
            }
            url::Url::parse(line)
                .ok()
                .and_then(|url| url.to_file_path().ok())
                .or_else(|| Path::new(line).is_absolute().then(|| PathBuf::from(line)))
        })
        .filter(|path| path.symlink_metadata().is_ok())
        .collect()
}

pub fn create(path: &Path, folder: bool) -> io::Result<()> {
    if folder {
        fs::create_dir(path)
    } else {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map(|_| ())
    }
}

pub fn create_link(source: &Path, destination: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, destination)
}

#[cfg(test)]
pub fn paste(sources: &[PathBuf], directory: &Path, link: bool) -> io::Result<()> {
    paste_with_receipt(sources, directory, link).map(|_| ())
}

#[cfg(test)]
pub fn paste_with_receipt(sources: &[PathBuf], directory: &Path, link: bool) -> io::Result<Vec<PathBuf>> {
    let mut created = Vec::new();
    for source in sources {
        let name = source
            .file_name()
            .ok_or_else(|| io::Error::other("Source has no file name"))?;
        let mut target = directory.join(name);
        // Copying an item into its own folder is the file manager's Duplicate
        // operation. Keep the original and choose a stable, human-readable name.
        let same_directory = source
            .parent()
            .and_then(|parent| parent.canonicalize().ok())
            .zip(directory.canonicalize().ok())
            .is_some_and(|(source_parent, destination)| source_parent == destination);
        if same_directory && target.symlink_metadata().is_ok() {
            target = duplicate_name(directory, name);
        }
        // Never overwrite an existing file or follow an existing destination link.
        if target.symlink_metadata().is_ok() {
            return Err(io::Error::other(format!(
                "{} already exists",
                target.display()
            )));
        }
        if source.is_dir()
            && directory
                .canonicalize()?
                .starts_with(fs::canonicalize(source)?)
        {
            return Err(io::Error::other("Cannot paste a folder inside itself"));
        }
        if link {
            std::os::unix::fs::symlink(source, &target)?;
        } else {
            copy(source, &target)?;
        }
        created.push(target);
    }
    Ok(created)
}

#[derive(Clone, Copy)]
pub enum ConflictPolicy { Replace, Skip, KeepBoth }

pub struct PasteReceipt {
    pub created: Vec<PathBuf>,
    pub pairs: Vec<(PathBuf, PathBuf)>,
    pub replaced: Vec<PathBuf>,
}

pub fn has_conflicts(sources: &[PathBuf], directory: &Path) -> bool {
    let mut names = std::collections::HashSet::new();
    sources.iter().any(|source| {
        let Some(name) = source.file_name() else { return true; };
        let same_directory = source.parent().and_then(|p| p.canonicalize().ok())
            .zip(directory.canonicalize().ok()).is_some_and(|(a,b)| a == b);
        let repeated_name = !same_directory && !names.insert(name.to_os_string());
        let existing_target = !same_directory && directory.join(name).symlink_metadata().is_ok();
        repeated_name || existing_target
    })
}

pub fn paste_resolving(
    sources: &[PathBuf],
    directory: &Path,
    link: bool,
    policy: ConflictPolicy,
    progress: &mut dyn FnMut(usize, usize, &Path),
) -> io::Result<PasteReceipt> {
    let mut receipt = PasteReceipt { created: Vec::new(), pairs: Vec::new(), replaced: Vec::new() };
    for (index, source) in sources.iter().enumerate() {
        progress(index, sources.len(), source);
        let name = source.file_name().ok_or_else(|| io::Error::other("Source has no file name"))?;
        let mut target = directory.join(name);
        let same_directory = source.parent().and_then(|p| p.canonicalize().ok())
            .zip(directory.canonicalize().ok()).is_some_and(|(a,b)| a == b);
        if source.is_dir() && directory.canonicalize()?.starts_with(fs::canonicalize(source)?) {
            return Err(io::Error::other("Cannot paste a folder inside itself"));
        }
        if target.symlink_metadata().is_ok() {
            if same_directory {
                target = duplicate_name(directory, name);
            } else {
                match policy {
                    ConflictPolicy::Skip => { progress(index + 1, sources.len(), source); continue; },
                    ConflictPolicy::KeepBoth => target = duplicate_name(directory, name),
                    ConflictPolicy::Replace => {
                        if let Err(error) = trash(std::slice::from_ref(&target)) {
                            for created in &receipt.created { let _ = delete(std::slice::from_ref(created)); }
                            let _ = restore_originals(&receipt.replaced);
                            return Err(error);
                        }
                        receipt.replaced.push(target.clone());
                    }
                }
            }
        }
        let result = if link { std::os::unix::fs::symlink(source, &target) } else { copy(source, &target) };
        if let Err(error) = result {
            let _ = delete(std::slice::from_ref(&target));
            for created in &receipt.created { let _ = delete(std::slice::from_ref(created)); }
            let _ = restore_originals(&receipt.replaced);
            return Err(error);
        }
        receipt.pairs.push((source.clone(), target.clone()));
        receipt.created.push(target);
        progress(index + 1, sources.len(), source);
    }
    Ok(receipt)
}

pub fn move_resolving(sources: &[PathBuf], directory: &Path, policy: ConflictPolicy, progress: &mut dyn FnMut(usize, usize, &Path)) -> io::Result<(Vec<(PathBuf, PathBuf)>, Vec<PathBuf>)> {
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut replaced = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        progress(index, sources.len(), source);
        let name = source.file_name().ok_or_else(|| io::Error::other("Source has no name"))?;
        let mut target = directory.join(name);
        let same_directory = source.parent().and_then(|p| p.canonicalize().ok())
            .zip(directory.canonicalize().ok()).is_some_and(|(a,b)| a == b);
        if same_directory { progress(index + 1, sources.len(), source); continue; }
        if target.symlink_metadata().is_ok() {
            match policy {
                ConflictPolicy::Skip => { progress(index + 1, sources.len(), source); continue; },
                ConflictPolicy::KeepBoth => target = duplicate_name(directory, name),
                ConflictPolicy::Replace => {
                    if let Err(error) = trash(std::slice::from_ref(&target)) {
                        for (from, to) in moved.iter().rev() { let _ = move_exact(to, from); }
                        let _ = restore_originals(&replaced);
                        return Err(error);
                    }
                    replaced.push(target.clone());
                }
            }
        }
        if source.is_dir() && target.starts_with(source) {
            let _ = restore_originals(&replaced);
            return Err(io::Error::other("Cannot move a folder into itself or one of its subfolders"));
        }
        if let Err(error) = move_exact(source, &target) {
            for (from, to) in moved.iter().rev() { let _ = move_exact(to, from); }
            let _ = restore_originals(&replaced);
            return Err(error);
        }
        moved.push((source.clone(), target));
        progress(index + 1, sources.len(), source);
    }
    Ok((moved, replaced))
}

fn duplicate_name(directory: &Path, name: &std::ffi::OsStr) -> PathBuf {
    let original = Path::new(name);
    let stem = original.file_stem().unwrap_or(name).to_string_lossy();
    let extension = original.extension().map(|ext| ext.to_string_lossy());
    for index in 1u32.. {
        let suffix = if index == 1 { " (copy)".to_owned() } else { format!(" (copy {index})") };
        let filename = match &extension {
            Some(extension) => format!("{stem}{suffix}.{extension}"),
            None => format!("{stem}{suffix}"),
        };
        let candidate = directory.join(filename);
        if candidate.symlink_metadata().is_err() {
            return candidate;
        }
    }
    unreachable!("u32 duplicate name space exhausted")
}

pub fn keep_both_name(directory: &Path, name: &std::ffi::OsStr) -> PathBuf {
    duplicate_name(directory, name)
}

fn copy(source: &Path, target: &Path) -> io::Result<()> {
    let metadata = source.symlink_metadata()?;
    if metadata.file_type().is_symlink() {
        std::os::unix::fs::symlink(fs::read_link(source)?, target)
    } else if metadata.is_dir() {
        fs::create_dir(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy(&entry.path(), &target.join(entry.file_name()))?;
        }
        Ok(())
    } else if metadata.is_file() {
        let mut input = fs::File::open(source)?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?;
        io::copy(&mut input, &mut output)?;
        fs::set_permissions(target, metadata.permissions())
    } else {
        Err(io::Error::other("Cannot copy this type of file"))
    }
}

/// Names stay within the chosen folder; absolute paths and traversal are rejected.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\0')
    {
        Err("Enter a name without slashes; . and .. are not valid names.".into())
    } else {
        Ok(())
    }
}

/// Linux renameat2 guarantees that a concurrent destination cannot be overwritten.
pub fn rename(source: &Path, target: &Path) -> io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    if source == target {
        return Ok(());
    }
    use std::os::unix::fs::MetadataExt;
    let same_entry = source.symlink_metadata().ok().zip(target.symlink_metadata().ok())
        .is_some_and(|(a, b)| a.dev() == b.dev() && a.ino() == b.ino());
    if same_entry {
        let parent = source.parent().unwrap_or_else(|| Path::new("."));
        let mut temp = None;
        for nonce in 0..1000u32 {
            let candidate = parent.join(format!(".veyra-rename-{}-{nonce}", std::process::id()));
            if candidate.symlink_metadata().is_err() { temp = Some(candidate); break; }
        }
        let temp = temp.ok_or_else(|| io::Error::other("Could not allocate a temporary filename"))?;
        rename(source, &temp)?;
        if let Err(error) = rename(&temp, target) {
            let restore = rename(&temp, source);
            return match restore { Ok(()) => Err(error), Err(restore_error) => Err(io::Error::other(format!("{error}; restoring original name also failed: {restore_error}"))) };
        }
        return Ok(());
    }
    let source = CString::new(source.as_os_str().as_bytes())?;
    let target = CString::new(target.as_os_str().as_bytes())?;
    // Both C strings remain alive for the call; paths are relative to AT_FDCWD.
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
#[cfg(test)]
pub fn move_to(sources: &[PathBuf], directory: &Path) -> io::Result<()> {
    let destination = directory.canonicalize()?;
    for source in sources {
        if source.parent().is_some_and(|parent| parent.canonicalize().ok().as_deref() == Some(destination.as_path())) {
            continue;
        }
        if source.is_dir() && destination.starts_with(source.canonicalize()?) {
            return Err(io::Error::other("Cannot move a folder into itself or one of its subfolders"));
        }
        let target = directory.join(
            source
                .file_name()
                .ok_or_else(|| io::Error::other("Source has no name"))?,
        );
        match rename(source, &target) {
            Ok(()) => {}
            Err(error) if error.raw_os_error() == Some(libc::EXDEV) => {
                paste(&[source.clone()], directory, false)?;
                delete(&[source.clone()])?;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
pub fn delete(paths: &[PathBuf]) -> io::Result<()> {
    delete_with_progress(paths, &mut |_, _, _| {})
}

pub fn delete_with_progress(paths: &[PathBuf], progress: &mut dyn FnMut(usize, usize, &Path)) -> io::Result<()> {
    for (index, path) in paths.iter().enumerate() {
        progress(index, paths.len(), path);
        let metadata = path.symlink_metadata()?;
        if metadata.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        progress(index + 1, paths.len(), path);
    }
    Ok(())
}
pub fn trash(paths: &[PathBuf]) -> io::Result<()> {
    trash_with_progress(paths, &mut |_, _, _| {})
}

pub fn trash_with_progress(paths: &[PathBuf], progress: &mut dyn FnMut(usize, usize, &Path)) -> io::Result<()> {
    // GIO implements the desktop trash spec, including mounted volumes.
    for (index, path) in paths.iter().enumerate() {
        progress(index, paths.len(), path);
        let output = std::process::Command::new("gio").arg("trash").arg("--").arg(path).output()?;
        if !output.status.success() {
            return Err(io::Error::other(String::from_utf8_lossy(&output.stderr).trim().to_owned()));
        }
        progress(index + 1, paths.len(), path);
    }
    Ok(())
}

pub const TRASH_URI: &str = "trash:///";

#[derive(Clone)]
pub enum HistoryAction {
    Remove(Vec<PathBuf>),
    Move(Vec<(PathBuf, PathBuf)>),
    Copy(Vec<(PathBuf, PathBuf)>),
    Link(Vec<(PathBuf, PathBuf)>),
    Create { path: PathBuf, folder: bool },
    Extract { archive: PathBuf, destination: PathBuf },
    Trash(Vec<PathBuf>),
    Restore(Vec<PathBuf>),
    Sequence(Vec<HistoryAction>),
}

#[derive(Clone)]
pub struct HistoryRecord {
    pub undo: HistoryAction,
    pub redo: HistoryAction,
}

pub fn apply_history_action(action: &HistoryAction) -> io::Result<()> {
    match action {
        HistoryAction::Remove(paths) => {
            for path in paths {
                if path.to_string_lossy().starts_with("trash://") {
                    delete_trash_file(&gio::File::for_uri(&path.to_string_lossy()))?;
                } else {
                    delete(&[path.clone()])?;
                }
            }
            Ok(())
        }
        HistoryAction::Move(pairs) => {
            for (source, destination) in pairs {
                move_exact(source, destination)?;
            }
            Ok(())
        }
        HistoryAction::Copy(pairs) => {
            for (source, destination) in pairs {
                copy(source, destination)?;
            }
            Ok(())
        }
        HistoryAction::Link(pairs) => {
            for (source, destination) in pairs {
                std::os::unix::fs::symlink(source, destination)?;
            }
            Ok(())
        }
        HistoryAction::Create { path, folder } => create(path, *folder),
        HistoryAction::Extract { archive, destination } => extract_zip_to_folder(archive, destination)
            .map_err(io::Error::other),
        HistoryAction::Trash(paths) => trash(paths),
        HistoryAction::Restore(paths) => restore_originals(paths),
        HistoryAction::Sequence(actions) => {
            for action in actions { apply_history_action(action)?; }
            Ok(())
        }
    }
}

fn move_exact(source: &Path, destination: &Path) -> io::Result<()> {
    use gio::prelude::*;
    if source.to_string_lossy().starts_with("trash://") {
        return gio::File::for_uri(&source.to_string_lossy())
            .move_(
                &gio::File::for_path(destination),
                gio::FileCopyFlags::NONE,
                None::<&gio::Cancellable>,
                None,
            )
            .map_err(|error| io::Error::other(error.to_string()));
    }
    match rename(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if error.raw_os_error() == Some(libc::EXDEV) => {
            copy(source, destination)?;
            delete(&[source.to_path_buf()])
        }
        Err(error) => Err(error),
    }
}

pub fn restore_originals(originals: &[PathBuf]) -> io::Result<()> {
    restore_originals_with_progress(originals, &mut |_, _, _| {})
}

pub fn restore_originals_with_progress(originals: &[PathBuf], progress: &mut dyn FnMut(usize, usize, &Path)) -> io::Result<()> {
    use gio::prelude::*;
    let entries = read_trash()?;
    for (index, original) in originals.iter().enumerate() {
        progress(index, originals.len(), original);
        let original_name = original.to_string_lossy();
        let trashed = entries.iter().find_map(|entry| {
            let file = gio::File::for_uri(&entry.to_string_lossy());
            let info = file
                .query_info("trash::orig-path", gio::FileQueryInfoFlags::NONE, None::<&gio::Cancellable>)
                .ok()?;
            (info.attribute_byte_string("trash::orig-path").as_deref() == Some(original_name.as_ref()))
                .then_some(file)
        }).ok_or_else(|| io::Error::other(format!("No Trash entry for {}", original.display())))?;
        trashed
            .move_(&gio::File::for_path(original), gio::FileCopyFlags::NONE, None::<&gio::Cancellable>, None)
            .map_err(|error| io::Error::other(error.to_string()))?;
        progress(index + 1, originals.len(), original);
    }
    Ok(())
}

pub fn is_trash_location(path: &Path) -> bool {
    path == Path::new(TRASH_URI)
}

/// Return trash children as GIO URIs so the trash backend remains in control
/// of files from both the home trash and mounted-volume trash locations.
pub fn read_trash() -> io::Result<Vec<PathBuf>> {
    use gio::prelude::*;
    let trash = gio::File::for_uri(TRASH_URI);
    let enumerator = trash
        .enumerate_children("standard::name", gio::FileQueryInfoFlags::NONE, None::<&gio::Cancellable>)
        .map_err(|error| io::Error::other(error.to_string()))?;
    let mut entries = Vec::new();
    while let Some(info) = enumerator
        .next_file(None::<&gio::Cancellable>)
        .map_err(|error| io::Error::other(error.to_string()))?
    {
        let child = trash.child(&info.name());
        entries.push(PathBuf::from(child.uri().as_str()));
    }
    entries.sort();
    Ok(entries)
}

#[cfg(test)]
pub fn restore_from_trash(paths: &[PathBuf]) -> io::Result<()> {
    let originals = originals_for_trash(paths)?;
    restore_originals(&originals)
}

pub fn originals_for_trash(paths: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    use gio::prelude::*;
    paths.iter().map(|path| {
        let trashed = gio::File::for_uri(&path.to_string_lossy());
        let info = trashed
            .query_info("trash::orig-path", gio::FileQueryInfoFlags::NONE, None::<&gio::Cancellable>)
            .map_err(|error| io::Error::other(error.to_string()))?;
        let original = info
            .attribute_byte_string("trash::orig-path")
            .ok_or_else(|| io::Error::other("Trash entry has no original location"))?;
        Ok(PathBuf::from(original.as_str()))
    }).collect()
}

#[cfg(test)]
pub fn empty_trash() -> io::Result<()> {
    empty_trash_with_progress(&mut |_, _, _| {})
}

pub fn empty_trash_with_progress(progress: &mut dyn FnMut(usize, usize, &Path)) -> io::Result<()> {
    let paths = read_trash()?;
    for (index, path) in paths.iter().enumerate() {
        progress(index, paths.len(), path);
        delete_trash_file(&gio::File::for_uri(&path.to_string_lossy()))?;
        progress(index + 1, paths.len(), path);
    }
    Ok(())
}

#[cfg(test)]
pub fn delete_from_trash(paths: &[PathBuf]) -> io::Result<()> {
    delete_from_trash_with_progress(paths, &mut |_, _, _| {})
}

pub fn delete_from_trash_with_progress(paths: &[PathBuf], progress: &mut dyn FnMut(usize, usize, &Path)) -> io::Result<()> {
    for (index, path) in paths.iter().enumerate() {
        progress(index, paths.len(), path);
        delete_trash_file(&gio::File::for_uri(&path.to_string_lossy()))?;
        progress(index + 1, paths.len(), path);
    }
    Ok(())
}

fn delete_trash_file(file: &gio::File) -> io::Result<()> {
    use gio::prelude::*;
    if let Ok(enumerator) = file.enumerate_children(
        "standard::type,standard::name",
        gio::FileQueryInfoFlags::NONE,
        None::<&gio::Cancellable>,
    ) {
        while let Some(info) = enumerator
            .next_file(None::<&gio::Cancellable>)
            .map_err(|error| io::Error::other(error.to_string()))?
        {
            delete_trash_file(&file.child(&info.name()))?;
        }
    }
    file.delete(None::<&gio::Cancellable>)
        .map_err(|error| io::Error::other(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "veyra-menu-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn paste_copies_nested_files_and_refuses_overwrites_and_recursive_destinations() {
        let temp = Directory::new();
        let source = temp.0.join("source");
        let destination = temp.0.join("destination");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::create_dir(&destination).unwrap();
        fs::write(source.join("nested/file.txt"), "original").unwrap();
        paste(&[source.clone()], &destination, false).unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("source/nested/file.txt")).unwrap(),
            "original"
        );
        assert!(paste(&[source.clone()], &destination, false).is_err());
        assert!(paste(&[source.clone()], &source.join("nested"), false).is_err());
        assert!(create(&source.join("nested/file.txt"), false).is_err());
        assert_eq!(
            fs::read_to_string(source.join("nested/file.txt")).unwrap(),
            "original"
        );
    }

    #[test]
    fn clipboard_file_urls_and_paste_links_preserve_paths_with_spaces() {
        let temp = Directory::new();
        let source = temp.0.join("with spaces.txt");
        fs::write(&source, "content").unwrap();
        let url = url::Url::from_file_path(&source).unwrap();
        assert_eq!(
            clipboard_paths(&format!("# comment\n{url}\nhttps://example.com")),
            vec![source.clone()]
        );
        let destination = temp.0.join("destination");
        fs::create_dir(&destination).unwrap();
        paste(&[source.clone()], &destination, true).unwrap();
        assert_eq!(
            fs::read_link(destination.join("with spaces.txt")).unwrap(),
            source
        );
    }
    #[test]
    fn create_rename_move_and_delete_work_without_overwriting() {
        let temp = Directory::new();
        let folder = temp.0.join("New folder");
        create(&folder, true).unwrap();
        let source = folder.join("file.txt");
        create(&source, false).unwrap();
        fs::write(&source, "editable content").unwrap();
        let renamed = folder.join("renamed.txt");
        rename(&source, &renamed).unwrap();
        assert!(!source.exists());
        let occupied = folder.join("occupied.txt");
        fs::write(&occupied, "keep me").unwrap();
        assert!(rename(&renamed, &occupied).is_err());
        assert_eq!(fs::read_to_string(&occupied).unwrap(), "keep me");
        move_to(&[renamed.clone()], &temp.0).unwrap();
        assert!(!renamed.exists());
        let moved = temp.0.join("renamed.txt");
        assert_eq!(fs::read_to_string(&moved).unwrap(), "editable content");
        delete(&[folder.clone()]).unwrap();
        assert!(!folder.exists());
        assert!(moved.exists());
        for name in ["", " ", ".", "..", "../escape", "/absolute"] {
            assert!(validate_name(name).is_err());
        }
        assert!(validate_name("A real folder").is_ok());
    }

    #[test]
    fn copying_and_deleting_links_does_not_modify_their_targets() {
        let temp = Directory::new();
        let source = temp.0.join("link");
        let target = temp.0.join("target");
        create(&target, true).unwrap();
        fs::write(target.join("keep.txt"), "keep").unwrap();
        std::os::unix::fs::symlink(&target, &source).unwrap();
        let destination = temp.0.join("destination");
        create(&destination, true).unwrap();
        paste(&[source.clone()], &destination, false).unwrap();
        assert!(
            destination
                .join("link")
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
        delete(&[source, destination.join("link")]).unwrap();
        assert_eq!(fs::read_to_string(target.join("keep.txt")).unwrap(), "keep");
    }
}
pub fn is_zip(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

/// Extract an archive into a new sibling directory. Refuse to overwrite an
/// existing destination, and remove a partial result if extraction fails.
pub fn extract_zip_to_folder(archive_path: &Path, destination: &Path) -> Result<(), String> {
    if !is_zip(archive_path) { return Err("This file is not a ZIP archive.".into()); }
    if destination.exists() { return Err(format!("{} already exists.", destination.display())); }
    std::fs::create_dir(destination).map_err(|error| format!("Could not create {}: {error}", destination.display()))?;
    let result = fs::File::open(archive_path)
        .map_err(|error| format!("Could not open {}: {error}", archive_path.display()))
        .and_then(|file| zip::ZipArchive::new(file).map_err(|error| format!("Invalid ZIP archive: {error}")))
        .and_then(|mut archive| archive.extract(destination).map_err(|error| format!("Could not extract ZIP archive: {error}")));
    if let Err(error) = result {
        let _ = std::fs::remove_dir_all(destination);
        return Err(error);
    }
    Ok(())
}

pub fn zip_destination(path: &Path) -> PathBuf {
    let name = path.file_stem().unwrap_or_default();
    path.parent().unwrap_or_else(|| Path::new(".")).join(name)
}
