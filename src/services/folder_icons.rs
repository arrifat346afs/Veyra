//! Persistent custom icons keyed by their folder paths.
use std::{collections::HashMap, env, fs, io, path::{Path, PathBuf}, sync::{OnceLock, RwLock}};

fn storage_path() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".config"))
        .join("veyra/folder-icons.json")
}

fn icons() -> &'static RwLock<HashMap<PathBuf, PathBuf>> {
    static ICONS: OnceLock<RwLock<HashMap<PathBuf, PathBuf>>> = OnceLock::new();
    ICONS.get_or_init(|| {
        let map = fs::read(storage_path()).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
        RwLock::new(map)
    })
}

fn normalize(path: &Path) -> PathBuf { path.canonicalize().unwrap_or_else(|_| path.to_path_buf()) }

pub fn get(folder: &Path) -> Option<PathBuf> { icons().read().ok()?.get(&normalize(folder)).cloned() }

pub fn set(folder: &Path, icon: &Path) -> io::Result<()> {
    if !folder.is_dir() { return Err(io::Error::new(io::ErrorKind::InvalidInput, "Select an existing folder.")); }
    if !icon.is_file() { return Err(io::Error::new(io::ErrorKind::InvalidInput, "Select an existing image file.")); }
    let is_svg = icon.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
        && fs::metadata(icon).is_ok_and(|metadata| metadata.len() <= 16 * 1024 * 1024)
        && fs::read_to_string(icon).is_ok_and(|svg| roxmltree::Document::parse(&svg)
            .is_ok_and(|document| document.root_element().tag_name().name() == "svg"));
    if !is_svg && image::image_dimensions(icon).is_err() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "The selected file is not a supported image."));
    }
    let mut guard = icons().write().map_err(|_| io::Error::other("Folder icon settings are unavailable."))?;
    let mut updated = guard.clone();
    updated.insert(normalize(folder), normalize(icon));
    save(&updated)?;
    *guard = updated;
    Ok(())
}

pub fn remove(folder: &Path) -> io::Result<()> {
    let mut guard = icons().write().map_err(|_| io::Error::other("Folder icon settings are unavailable."))?;
    let mut updated = guard.clone();
    updated.remove(&normalize(folder));
    save(&updated)?;
    *guard = updated;
    Ok(())
}

pub fn relocate(source: &Path, target: &Path) -> io::Result<()> {
    let target = normalize(target);
    let mut guard = icons().write().map_err(|_| io::Error::other("Folder icon settings are unavailable."))?;
    let updated = relocated(&guard, source, &target);
    if *guard != updated {
        save(&updated)?;
        *guard = updated;
    }
    Ok(())
}

pub fn duplicate(source: &Path, target: &Path) -> io::Result<()> {
    let source = normalize(source);
    let target = normalize(target);
    let mut guard = icons().write().map_err(|_| io::Error::other("Folder icon settings are unavailable."))?;
    let mut updated = guard.clone();
    for (folder, icon) in guard.iter() {
        let Ok(suffix) = folder.strip_prefix(&source) else { continue; };
        let copied_icon = icon.strip_prefix(&source).map(|suffix| target.join(suffix)).unwrap_or_else(|_| icon.clone());
        updated.insert(target.join(suffix), copied_icon);
    }
    if *guard != updated {
        save(&updated)?;
        *guard = updated;
    }
    Ok(())
}

fn relocated(icons: &HashMap<PathBuf, PathBuf>, source: &Path, target: &Path) -> HashMap<PathBuf, PathBuf> {
    let mut updated = icons.clone();
    let moving_folder = icons.keys().any(|folder| folder.starts_with(source));
    let moved: Vec<_> = icons.iter().filter_map(|(folder, icon)| {
        let new_folder = folder.strip_prefix(source).map(|suffix| target.join(suffix)).unwrap_or_else(|_| folder.clone());
        let new_icon = icon.strip_prefix(source).map(|suffix| target.join(suffix)).unwrap_or_else(|_| icon.clone());
        (new_folder != *folder || new_icon != *icon).then_some((folder.clone(), new_folder, new_icon))
    }).collect();
    for (old, _, _) in &moved { updated.remove(old); }
    if moving_folder { updated.retain(|folder, _| !folder.starts_with(target)); }
    for (_, folder, icon) in moved { updated.insert(folder, icon); }
    updated
}

fn save(icons: &HashMap<PathBuf, PathBuf>) -> io::Result<()> {
    let file = storage_path();
    let parent = file.parent().unwrap();
    fs::create_dir_all(parent)?;
    let temporary = file.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(icons).map_err(io::Error::other)?)?;
    fs::rename(temporary, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_a_folder_keeps_icons_for_it_and_its_children() {
        let icons = HashMap::from([
            (PathBuf::from("/old"), PathBuf::from("/old/icon.png")),
            (PathBuf::from("/old/child"), PathBuf::from("/elsewhere/icon.png")),
            (PathBuf::from("/new"), PathBuf::from("/stale.png")),
        ]);
        let moved = relocated(&icons, Path::new("/old"), Path::new("/new"));
        assert_eq!(moved.get(Path::new("/new")), Some(&PathBuf::from("/new/icon.png")));
        assert_eq!(moved.get(Path::new("/new/child")), Some(&PathBuf::from("/elsewhere/icon.png")));
        assert!(!moved.contains_key(Path::new("/old")));
    }
}
