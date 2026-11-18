//! Editable location state and path resolution, reusable by the path bar view.
use std::path::{Path, PathBuf};
use gpui_kit::*;
use gpui_kit::component::input::InputState;

pub struct PathBar {
    pub input: Entity<InputState>,
    pub editing: bool,
    pub error: Option<String>,
    pub suggestions: Vec<PathBuf>,
    pub highlighted: Option<usize>,
    pub request_id: u64,
}
impl PathBar {
    pub fn new(window: &mut Window, cx: &mut Context<crate::windows::MainWindow>) -> Self {
        Self { input: cx.new(|cx| InputState::new(window, cx).placeholder("Enter a folder path")), editing: false, error: None,
            suggestions: Vec::new(), highlighted: None, request_id: 0 }
    }
    pub fn start(&mut self, current: &Path, window: &mut Window, cx: &mut App) {
        let input = self.input.clone();
        input.update(cx, |input, cx| {
            input.set_value(current.to_string_lossy().to_string(), window, cx);
            input.focus(window, cx);
        });
        self.error = None;
        self.suggestions.clear();
        self.highlighted = None;
        self.request_id = self.request_id.wrapping_add(1);
        self.editing = true;
    }
    pub fn cancel(&mut self) { self.editing = false; self.error = None; self.suggestions.clear(); self.highlighted = None;
        self.request_id = self.request_id.wrapping_add(1); }
    pub fn accept(&mut self, index: usize, window: &mut Window, cx: &mut App) {
        let Some(path) = self.suggestions.get(index) else { return; };
        let mut value = path.to_string_lossy().to_string();
        value.push(std::path::MAIN_SEPARATOR);
        self.input.update(cx, |input, cx| { input.set_value(value, window, cx); input.focus(window, cx); });
        self.suggestions.clear();
        self.highlighted = None;
        self.error = None;
        self.request_id = self.request_id.wrapping_add(1);
    }
    pub fn resolve(&self, current: &Path, cx: &App) -> Result<PathBuf, String> {
        let raw = self.input.read(cx).value();
        resolve_path(raw.trim(), current)
    }
}

fn resolve_path(raw: &str, current: &Path) -> Result<PathBuf, String> {
        if raw.is_empty() { return Err("Enter a folder path".into()); }
        let target = if raw == "~" || raw.starts_with("~/") {
            let home = std::env::var_os("HOME").ok_or("Home directory is unavailable")?;
            PathBuf::from(home).join(raw.trim_start_matches('~').trim_start_matches('/'))
        } else {
            let path = PathBuf::from(raw);
            if path.is_absolute() {path} else {current.join(path)}
        };
        if target.is_dir() { Ok(target.canonicalize().unwrap_or(target)) }
        else { Err(format!("Folder not found: {}", target.display())) }
}

#[cfg(test)]
mod tests {
    use super::resolve_path;
    #[test]
    fn accepts_absolute_and_relative_folders_and_rejects_missing_ones() {
        let base = std::env::temp_dir();
        assert_eq!(resolve_path(base.to_str().unwrap(), &base).unwrap(), base.canonicalize().unwrap());
        assert_eq!(resolve_path("..", &base).unwrap(), base.parent().unwrap().canonicalize().unwrap());
        assert!(resolve_path("", &base).is_err());
        assert!(resolve_path("a-folder-that-does-not-exist-in-veyra", &base).is_err());
    }
}
