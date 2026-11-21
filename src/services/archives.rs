//! ZIP and TAR creation plus extraction through installed archive tools.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy, Debug)]
pub enum Format { Zip, Tar }

impl Format {
    pub fn extension(self) -> &'static str { match self { Self::Zip => "zip", Self::Tar => "tar" } }
}

pub fn is_archive(path: &Path) -> bool {
    if !path.is_file() { return false; }
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
    [".zip", ".tar", ".tar.gz", ".tgz", ".tar.bz2", ".tbz", ".tbz2", ".tar.xz", ".txz", ".tar.zst", ".tzst", ".tar.lz", ".tlz", ".tar.lzma", ".tar.z", ".taz", ".7z", ".rar", ".cab", ".iso", ".cpio", ".ar", ".deb", ".rpm", ".jar", ".war", ".apk", ".xar", ".lha", ".lzh", ".gz", ".bz2", ".xz", ".zst", ".lz4", ".lzma"]
        .iter().any(|suffix| name.ends_with(suffix))
}

pub fn default_destination(archive: &Path) -> PathBuf {
    let parent = archive.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let stem = archive_stem(archive);
    let mut candidate = parent.join(&stem);
    let mut suffix = 1;
    while candidate.symlink_metadata().is_ok() {
        candidate = parent.join(format!("{stem} ({suffix})"));
        suffix += 1;
    }
    candidate
}

pub fn with_extension(name: &str, format: Format) -> String {
    let extension = format!(".{}", format.extension());
    if name.to_ascii_lowercase().ends_with(&extension) { name.to_owned() }
    else { format!("{name}{extension}") }
}

pub fn default_name(sources: &[PathBuf], format: Format) -> String {
    let name = if sources.len() == 1 {
        sources[0].file_name().unwrap_or_default().to_string_lossy().into_owned()
    } else { "Archive".into() };
    with_extension(&name, format)
}

pub fn create(sources: &[PathBuf], output: &Path, format: Format) -> Result<(), String> {
    if sources.is_empty() { return Err("Select at least one file or folder to compress.".into()); }
    if output.symlink_metadata().is_ok() { return Err(format!("{} already exists.", output.display())); }
    let base = common_parent(sources)?;
    for source in sources {
        let metadata = fs::symlink_metadata(source).map_err(|error| format!("{}: {error}", source.display()))?;
        if !source.starts_with(&base) || !metadata.is_file() && !metadata.is_dir() && !metadata.file_type().is_symlink() {
            return Err(format!("{} is not a local file or folder.", source.display()));
        }
    }
    match format {
        Format::Zip => create_zip(sources, &base, output),
        Format::Tar => create_tar(sources, &base, output),
    }
}

fn create_zip(sources: &[PathBuf], base: &Path, output: &Path) -> Result<(), String> {
    let result = (|| {
        let file = OpenOptions::new().write(true).create_new(true).open(output).map_err(|error| error.to_string())?;
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for source in sources { add_zip_path(&mut archive, source, base, options)?; }
        archive.finish().map_err(|error| error.to_string())?;
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(output); }
    result
}

fn add_zip_path<W: Write + io::Seek>(archive: &mut zip::ZipWriter<W>, path: &Path, base: &Path, options: zip::write::SimpleFileOptions) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let relative = path.strip_prefix(base).map_err(|error| error.to_string())?;
    let name = relative.to_string_lossy().replace('\\', "/");
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(path).map_err(|error| error.to_string())?;
        archive.add_symlink(name, target.to_string_lossy(), options).map_err(|error| error.to_string())?;
    } else if metadata.is_dir() {
        archive.add_directory(format!("{name}/"), options).map_err(|error| error.to_string())?;
        let mut children = fs::read_dir(path).map_err(|error| error.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children { add_zip_path(archive, &child.path(), base, options)?; }
    } else if metadata.is_file() {
        archive.start_file(name, options).map_err(|error| error.to_string())?;
        io::copy(&mut File::open(path).map_err(|error| error.to_string())?, archive).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn create_tar(sources: &[PathBuf], base: &Path, output: &Path) -> Result<(), String> {
    let executable = find_program(&["tar", "bsdtar"]).ok_or("TAR creation requires tar or bsdtar to be installed.")?;
    let result = (|| {
        OpenOptions::new().write(true).create_new(true).open(output).map_err(|error| error.to_string())?;
        let mut command = Command::new(executable);
        command.arg("-cf").arg(output).arg("-C").arg(base).arg("--");
        for source in sources {
            let relative = source.strip_prefix(base).map_err(|error| error.to_string())?;
            command.arg(if relative.to_string_lossy().starts_with('-') { PathBuf::from(".").join(relative) } else { relative.to_path_buf() });
        }
        let output_result = command.output().map_err(|error| error.to_string())?;
        if !output_result.status.success() {
            return Err(String::from_utf8_lossy(&output_result.stderr).trim().to_owned());
        }
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(output); }
    result
}

pub fn extract(archive: &Path, destination: &Path) -> Result<(), String> {
    if !is_archive(archive) { return Err("This archive format is not supported.".into()); }
    if destination.symlink_metadata().is_ok() { return Err(format!("{} already exists. Choose a new destination folder.", destination.display())); }
    let parent = destination.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() { return Err(format!("Destination parent {} does not exist.", parent.display())); }
    let staging = tempfile::Builder::new().prefix(".veyra-extract-").tempdir_in(parent).map_err(|error| error.to_string())?;
    let name = archive.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
    let result = if name.ends_with(".zip") { extract_zip(archive, staging.path()) }
        else if is_tar(&name) { extract_with_tar(archive, staging.path()) }
        else { extract_with_7z(archive, staging.path()) };
    result?;
    let staged_path = staging.keep();
    if let Err(error) = fs::rename(&staged_path, destination) {
        let _ = fs::remove_dir_all(&staged_path);
        return Err(format!("Could not move extracted files to {}: {error}", destination.display()));
    }
    Ok(())
}

fn extract_zip(archive: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| format!("Invalid ZIP archive: {error}"))?;
    archive.extract(destination).map_err(|error| format!("Could not extract ZIP archive: {error}"))
}

fn extract_with_tar(archive: &Path, destination: &Path) -> Result<(), String> {
    let executable = find_program(&["bsdtar", "tar"]).ok_or("TAR extraction requires tar or bsdtar to be installed.")?;
    run_backend(Command::new(executable).args(["-xf"]).arg(archive).arg("-C").arg(destination)
        .arg("--no-same-owner").arg("--no-same-permissions"), archive)
}

fn extract_with_7z(archive: &Path, destination: &Path) -> Result<(), String> {
    if let Some(executable) = find_program(&["7z", "7zz"]) {
        return run_backend(Command::new(executable).arg("x").arg("-y").arg("-bd")
            .arg(format!("-o{}", destination.display())).arg(archive), archive);
    }
    if let Some(executable) = find_program(&["bsdtar"]) {
        return run_backend(Command::new(executable).args(["-xf"]).arg(archive).arg("-C").arg(destination), archive);
    }
    Err("This format needs 7z or bsdtar. Install one of them and try again.".into())
}

fn run_backend(command: &mut Command, archive: &Path) -> Result<(), String> {
    let output = command.output().map_err(|error| error.to_string())?;
    if output.status.success() { Ok(()) }
    else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if detail.is_empty() { format!("Could not extract {}.", archive.display()) } else { detail })
    }
}

fn is_tar(name: &str) -> bool {
    [".tar", ".tar.gz", ".tgz", ".tar.bz2", ".tbz", ".tbz2", ".tar.xz", ".txz", ".tar.zst", ".tzst", ".tar.lz", ".tlz", ".tar.lzma", ".tar.z", ".taz"]
        .iter().any(|suffix| name.ends_with(suffix))
}

fn archive_stem(path: &Path) -> String {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let lower = name.to_ascii_lowercase();
    for suffix in [".tar.lzma", ".tar.gz", ".tgz", ".tar.bz2", ".tbz2", ".tbz", ".tar.xz", ".txz", ".tar.zst", ".tzst", ".tar.lz", ".tlz", ".tar.z", ".taz", ".zip", ".tar", ".7z", ".rar", ".cab", ".iso", ".cpio", ".ar", ".deb", ".rpm", ".jar", ".war", ".apk", ".xar", ".lha", ".lzh", ".gz", ".bz2", ".xz", ".zst", ".lz4", ".lzma"] {
        if lower.ends_with(suffix) { return name[..name.len() - suffix.len()].to_owned(); }
    }
    path.file_stem().unwrap_or_default().to_string_lossy().into_owned()
}

fn common_parent(sources: &[PathBuf]) -> Result<PathBuf, String> {
    let mut base = sources.first().and_then(|path| path.parent()).ok_or("The selected item has no parent folder.")?.to_path_buf();
    for source in sources.iter().skip(1) {
        while !source.starts_with(&base) {
            if !base.pop() { return Err("Selected items do not share a parent folder.".into()); }
        }
    }
    Ok(base)
}

pub fn destination_directory(sources: &[PathBuf]) -> Result<PathBuf, String> { common_parent(sources) }

fn find_program(names: &[&str]) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    names.iter().flat_map(|name| std::env::split_paths(&paths).map(move |dir| dir.join(name)))
        .find(|path| path.is_file())
}
