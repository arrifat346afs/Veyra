use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub show_thumbnails: bool,
    pub list_view: bool, pub show_hidden: bool, pub reverse_sort: bool,
    pub folders_first: bool, pub sort_by: usize, pub icon_size: usize,
    pub density: usize, pub filename_size: usize, pub show_sizes: bool,
    pub single_click: bool, pub case_sensitive: bool, pub show_sidebar: bool,
    pub light_theme: bool, pub home_start: bool,
    pub window_opacity: usize, pub sidebar_opacity: usize,
}
impl Default for Preferences {
    fn default() -> Self { Self { show_thumbnails:true, list_view:false, show_hidden:false, reverse_sort:false,
        folders_first:true, sort_by:0, icon_size:112, density:1, filename_size:12,
        show_sizes:true, single_click:true, case_sensitive:false, show_sidebar:true,
        light_theme:false, home_start:false, window_opacity:100, sidebar_opacity:100 } }
}
fn path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"))
        .join("veyra/settings.json")
}
impl Preferences {
    /// Shared alpha for raised grey surfaces throughout the interface.
    pub fn foreground_alpha(&self) -> f32 { self.sidebar_opacity.min(100) as f32 / 100. }

    pub fn load() -> Result<Self, String> {
        match fs::read(path()) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("Could not read settings: {e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("Could not read settings: {e}")),
        }
    }
    pub fn save(&self) -> Result<(), String> {
        let path = path();
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, bytes).and_then(|_| fs::rename(temp, path)).map_err(|e| format!("Could not save settings: {e}"))
    }
}
