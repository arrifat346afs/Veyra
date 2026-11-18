//! Persistent bookmarks owned by this application.
//!
//! The first version of the sidebar read `~/.config/gtk-3.0/bookmarks` every
//! time it rendered. That made the UI dependent on another file manager and
//! meant changes could not be made from this app. This module now stores its
//! own list in the XDG data directory. On the first run only, it imports the
//! GTK bookmark file as a convenience migration; later reads never consult it.

use std::{env, fs, io, path::{Path, PathBuf}};

/// One named filesystem location shown in the sidebar.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bookmark {
    /// User-facing label. Empty labels are replaced with the directory name.
    pub name: String,
    /// Absolute path to the bookmarked location.
    pub path: PathBuf,
}

/// Return this app's private bookmark file (`$XDG_DATA_HOME/filemanager/bookmarks`).
fn storage_path() -> Option<PathBuf> {
    let data_dir = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))?;
    Some(data_dir.join("filemanager").join("bookmarks"))
}

/// Read the saved bookmarks, initializing the app-owned file if necessary.
///
/// The storage format is intentionally plain text: one path per line. That
/// keeps it easy to inspect and recover by hand. Names are derived from the
/// final path component, so paths containing spaces need no custom escaping.
pub fn get_bookmarks() -> Vec<Bookmark> {
    let Some(file) = storage_path() else { return Vec::new() };
    if !file.exists() {
        // This is a one-time import, not a runtime dependency on GTK.
        let imported = import_gtk_bookmarks();
        let _ = save_to(&file, &imported);
    }
    fs::read_to_string(file).map(|text| parse(&text)).unwrap_or_default()
}

/// Add a directory to the private list, skipping duplicates.
pub fn add_bookmark(path: &Path) -> io::Result<()> {
    if !path.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Only existing folders can be bookmarked."));
    }
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut bookmarks = get_bookmarks();
    if bookmarks.iter().any(|bookmark| bookmark.path == path) { return Ok(()) }
    bookmarks.push(Bookmark { name: display_name(&path), path });
    save(&bookmarks)
}

/// Remove the bookmark pointing at `path`.
pub fn remove_bookmark(path: &Path) -> io::Result<()> {
    let mut bookmarks = get_bookmarks();
    bookmarks.retain(|bookmark| bookmark.path != path);
    save(&bookmarks)
}

/// Persist a complete bookmark list, creating the app data directory as needed.
fn save(bookmarks: &[Bookmark]) -> io::Result<()> {
    let Some(file) = storage_path() else {
        return Err(io::Error::new(io::ErrorKind::NotFound, "home data directory is unavailable"));
    };
    save_to(&file, bookmarks)
}

fn save_to(file: &Path, bookmarks: &[Bookmark]) -> io::Result<()> {
    if let Some(parent) = file.parent() { fs::create_dir_all(parent)?; }
    let body = bookmarks.iter().map(|bookmark| bookmark.path.to_string_lossy().into_owned())
        .collect::<Vec<_>>().join("\n");
    fs::write(file, body)
}

fn parse(text: &str) -> Vec<Bookmark> {
    text.lines().filter_map(|line| {
        let path = PathBuf::from(line.trim());
        if line.trim().is_empty() { return None; }
        Some(Bookmark { name: display_name(&path), path })
    }).collect()
}

fn display_name(path: &Path) -> String {
    path.file_name().and_then(|name| name.to_str()).filter(|name| !name.is_empty())
        .map(str::to_owned).unwrap_or_else(|| path.display().to_string())
}

/// Parse GTK's legacy bookmarks only for the first-run migration.
fn import_gtk_bookmarks() -> Vec<Bookmark> {
    let Some(config) = env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))) else {
        return Vec::new();
    };
    fs::read_to_string(config.join("gtk-3.0/bookmarks")).map(|text| {
        text.lines().filter_map(|line| {
            // GTK format is URI followed by an optional whitespace-delimited label.
            let uri = line.split_whitespace().next()?;
            let path = uri.strip_prefix("file://")?;
            let decoded = path.replace("%20", " ");
            let path = PathBuf::from(decoded);
            Some(Bookmark { name: display_name(&path), path })
        }).collect()
    }).unwrap_or_default()
}
