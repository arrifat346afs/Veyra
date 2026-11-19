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

pub fn paste(sources: &[PathBuf], directory: &Path, link: bool) -> io::Result<()> {
    for source in sources {
        let target = directory.join(
            source
                .file_name()
                .ok_or_else(|| io::Error::other("Source has no file name"))?,
        );
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
            std::os::unix::fs::symlink(source, target)?;
        } else {
            copy(source, &target)?;
        }
    }
    Ok(())
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
    for path in paths {
        let metadata = path.symlink_metadata()?;
        if metadata.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}
pub fn trash(paths: &[PathBuf]) -> io::Result<()> {
    // GIO implements the desktop trash spec, including mounted volumes.
    let output = std::process::Command::new("gio")
        .arg("trash")
        .arg("--")
        .args(paths)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
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
