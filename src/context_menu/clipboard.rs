//! File clipboard URI lists, including copy/cut markers used by Linux desktops.
use crate::windows::MainWindow;
use gpui_kit::*;
use std::path::PathBuf;
pub fn write(state: &mut MainWindow, paths: &[PathBuf], cut: bool, cx: &mut App) {
    let text = format!(
        "{}\n{}",
        if cut { "cut" } else { "copy" },
        paths
            .iter()
            .filter_map(|p| url::Url::from_file_path(p).ok())
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
    state.cut_clipboard = cut.then(|| text.clone());
    cx.write_to_clipboard(ClipboardItem::new_string(text));
}
pub fn read(state: &MainWindow, cx: &App) -> (Vec<PathBuf>, bool) {
    let text = cx
        .read_from_clipboard()
        .and_then(|i| i.text())
        .unwrap_or_default();
    let cut = text.lines().next() == Some("cut") || state.cut_clipboard.as_ref() == Some(&text);
    (super::files::clipboard_paths(&text), cut)
}
