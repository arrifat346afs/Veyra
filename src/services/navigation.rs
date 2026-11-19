//! Persistent recent and starred filesystem locations.
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

fn file(name: &str) -> Option<PathBuf> {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .map(|p| p.join("veyra").join(name))
}
fn load(name: &str) -> Vec<PathBuf> {
    file(name)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
fn save(name: &str, paths: &[PathBuf]) -> io::Result<()> {
    let p = file(name)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "data directory unavailable"))?;
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(p, serde_json::to_vec(paths).map_err(io::Error::other)?)
}
pub fn recent() -> Vec<PathBuf> {
    load("recent.json")
}
pub fn record_recent(path: &Path) {
    if !path.is_file() { return; }
    let mut paths = recent();
    paths.retain(|p| p != path);
    paths.insert(0, path.to_path_buf());
    paths.truncate(100);
    let _ = save("recent.json", &paths);
}
pub fn starred() -> Vec<PathBuf> {
    load("starred.json")
}
pub fn toggle_star(path: &Path) -> io::Result<()> {
    let mut paths = starred();
    if paths.iter().any(|p| p == path) {
        paths.retain(|p| p != path)
    } else {
        paths.insert(0, path.to_path_buf())
    }
    save("starred.json", &paths)
}
pub fn recent_location() -> PathBuf {
    PathBuf::from("veyra://recent")
}
pub fn starred_location() -> PathBuf {
    PathBuf::from("veyra://starred")
}
