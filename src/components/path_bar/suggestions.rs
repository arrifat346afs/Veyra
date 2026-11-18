//! Folder-name completion for the path segment currently being typed.
use std::path::{Path, PathBuf};

pub fn matching_folders(raw: &str, current: &Path) -> Vec<PathBuf> {
    let raw = raw.trim();
    if raw.is_empty() { return Vec::new(); }
    let ends_with_separator = raw.ends_with('/');
    let (parent_text, prefix) = if ends_with_separator {
        (raw.trim_end_matches('/'), "")
    } else if let Some((parent, name)) = raw.rsplit_once('/') {
        (parent, name)
    } else {
        ("", raw)
    };
    let parent = if raw.starts_with('/') {
        PathBuf::from(if parent_text.is_empty() {"/"} else {parent_text})
    } else if parent_text == "~" || parent_text.starts_with("~/") {
        let Some(home) = std::env::var_os("HOME") else {return Vec::new();};
        PathBuf::from(home).join(parent_text.trim_start_matches('~').trim_start_matches('/'))
    } else if parent_text.is_empty() {
        current.to_path_buf()
    } else {
        current.join(parent_text)
    };
    let Ok(entries) = std::fs::read_dir(parent) else {return Vec::new();};
    let prefix = prefix.to_lowercase();
    let mut paths: Vec<_> = entries.flatten().filter_map(|entry| {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_lowercase().starts_with(&prefix) || (name.starts_with('.') && !prefix.starts_with('.')) {
            return None;
        }
        entry.file_type().ok().filter(|kind| kind.is_dir() || kind.is_symlink())?;
        let path = entry.path();
        path.is_dir().then_some(path)
    }).collect();
    paths.sort_by_key(|path| path.file_name().map(|name| name.to_string_lossy().to_lowercase()));
    paths.truncate(10);
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completes_only_matching_folders() {
        let root = std::env::temp_dir().join(format!("veyra-path-suggestions-{}",std::process::id()));
        std::fs::create_dir_all(root.join("Documents")).unwrap();
        std::fs::create_dir_all(root.join("Downloads")).unwrap();
        std::fs::write(root.join("Document.txt"),b"file").unwrap();
        let matches = matching_folders("Do",&root);
        assert_eq!(matches.len(),2);
        assert!(matches.iter().all(|path| path.is_dir()));
        std::fs::remove_dir_all(root).unwrap();
    }
}
