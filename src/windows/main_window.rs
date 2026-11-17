//! Main file-manager window and its navigation state.
//!
//! `MainWindow` owns the current directory and the directory entries displayed
//! beside the sidebar. Sidebar rows call `navigate_to`; file rows call the same
//! method when they represent a directory. Keeping navigation in this window
//! makes built-in places and user bookmarks behave consistently.

use std::{fs, path::PathBuf};

use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::*;
use gpui_kit::component::Icon;
use gpui_kit_assets::IconName;

use crate::layouts::sidebar;

pub struct MainWindow {
    /// Directory whose contents are currently visible in the main pane.
    pub(crate) current_dir: PathBuf,
    /// Snapshot of the directory contents, refreshed after each navigation.
    entries: Vec<PathBuf>,
    /// Last filesystem error, shown in the content area instead of hiding it.
    error: Option<String>,
}

impl MainWindow {
    pub fn open(cx: &mut App) {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            window.set_window_title("Veyra");
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
            let (entries, error) = read_directory(&current_dir);
            cx.new(|_| Self { current_dir, entries, error })
        }).expect("Failed to open main window");
    }

    /// Switch the active location and reload its visible entries.
    pub(crate) fn navigate_to(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let (entries, error) = read_directory(&path);
        self.current_dir = path;
        self.entries = entries;
        self.error = error;
        cx.notify();
    }
}

/// Read a directory safely: inaccessible paths become a user-visible message.
fn read_directory(path: &PathBuf) -> (Vec<PathBuf>, Option<String>) {
    match fs::read_dir(path) {
        Ok(items) => {
            let mut entries: Vec<_> = items.filter_map(Result::ok).map(|item| item.path()).collect();
            // Directories first, then alphabetical order, which is predictable
            // for navigation and makes the basic browser easier to scan.
            entries.sort_by_key(|entry| (!entry.is_dir(), entry.file_name().unwrap_or_default().to_string_lossy().to_lowercase()));
            (entries, None)
        }
        Err(error) => (Vec::new(), Some(format!("{}: {error}", path.display()))),
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.theme().clone();
        let entry_rows: Vec<_> = self.entries.iter().map(|entry| {
            let path = entry.clone();
            let is_dir = entry.is_dir();
            let name = entry.file_name().and_then(|name| name.to_str()).unwrap_or("(unnamed)").to_owned();
            div().id(format!("entry-{}", entry.display())).h(px(38.0)).w_full().flex().items_center().gap(px(12.0)).px(px(10.0))
                .rounded(px(5.0)).text_color(palette.foreground).hover(|style| style.bg(palette.list_hover))
                        .child(div().text_color(palette.primary).child(Icon::new(
                            if is_dir { IconName::Folder } else { IconName::File }
                        ).size(px(16.0))))
                .child(name)
                .when(is_dir, |row| row.on_click(cx.listener(move |this, _, _, cx| this.navigate_to(path.clone(), cx))))
        }).collect();
        let content = div().flex_1().h_full().flex().flex_col().bg(palette.background)
            .child(div().h(px(52.0)).w_full().flex().items_center().px(px(20.0))
                .border_b_1().border_color(palette.border).text_color(palette.foreground)
                .child(self.current_dir.display().to_string()))
            .child(if let Some(error) = &self.error {
                div().px(px(20.0)).py(px(16.0)).text_color(palette.danger_foreground).child(error.clone())
            } else if self.entries.is_empty() {
                div().px(px(20.0)).py(px(16.0)).text_color(palette.muted_foreground).child("This folder is empty.")
            } else {
                div().flex().flex_col().p(px(12.0)).children(entry_rows)
            });

        let foreground = palette.foreground;
        // Build the sidebar after the content because its event handlers need a
        // mutable context; the layout itself remains in layouts/sidebar.
        let sidebar = sidebar::sidebar(cx);
        div().size_full().flex().text_color(foreground).child(sidebar).child(content)
    }
}
