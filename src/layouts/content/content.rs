//! Main directory content layout: breadcrumb, file grid, and file icons.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::prelude::*;

use crate::windows::MainWindow;

/// Render the active path and its file/folder entries in an icon grid.
pub fn content(
    _current_dir: &Path,
    entries: &[PathBuf],
    error: Option<&str>,
    list_view: bool,
    preferences: &crate::settings::preferences::Preferences,
    selection: &crate::selection::Selection,
    cx: &mut Context<MainWindow>,
) -> AnyElement {
    let icon_size = preferences.icon_size.clamp(48,192) as f32;
    let spacing = [20.,12.,8.,4.][preferences.density.min(3)];
    let single_click = preferences.single_click;
    let palette = cx.theme().clone();
    let entry_tiles: Vec<_> = entries.iter().map(|entry| {
        let path = entry.clone();
        let is_dir = entry.is_dir();
        let name = entry.file_name().and_then(|name| name.to_str()).unwrap_or("(unnamed)").to_owned();
        let icon = papirus_icon(entry, is_dir);
        let tile = div().id(format!("entry-{}", entry.display())).w(px(icon_size+52.)).h(px(icon_size+76.))
            .flex().flex_col().items_center().justify_start().gap(px(8.0)).p(px(10.0))
            .rounded(px(8.0)).text_color(palette.foreground).cursor_pointer()
            .when(list_view, |tile| tile.w_full().h(px(40.+spacing)).flex_row())
            // Load as an image so GPUI preserves Papirus' multicolor SVG fills.
            .child(crate::thumbnails::preview(entry, PathBuf::from(icon), !is_dir && preferences.show_thumbnails, if list_view {32.} else {icon_size}))
            .child(div().w_full().text_center().text_ellipsis().text_size(px(preferences.filename_size.clamp(10,24) as f32)).child(name))
            .when(!is_dir && preferences.show_sizes, |tile| tile.child(div().text_color(palette.muted_foreground).text_size(px(11.0))
                .child(format_file_size(entry))))
            .when(is_dir, |tile| tile.on_click(cx.listener(move |this, event: &ClickEvent, _, cx| { if !(event.modifiers().control || event.modifiers().shift) && (single_click || event.click_count() == 2) {this.navigate_to(path.clone(), cx)} })))
        ;
        crate::selection::item(tile, entry.clone(), selection, cx)
    }).collect();

    let body = div().flex_1().h_full().flex().flex_col().bg(palette.background.opacity(preferences.window_opacity.min(100) as f32 / 100.))
        .child(if let Some(error) = error {
            div().px(px(20.0)).py(px(16.0)).text_color(palette.danger_foreground).child(error.to_owned()).into_any_element()
        } else if entries.is_empty() {
            div().px(px(20.0)).py(px(16.0)).text_color(palette.muted_foreground).child("This folder is empty.").into_any_element()
        } else {
            div().flex_1().w_full().overflow_y_scrollbar().child(
                div().w_full().flex().flex_wrap().content_start().gap(px(spacing)).p(px(18.0)).children(entry_tiles)
            ).h_full().into_any_element()
        }).into_any_element();
    crate::selection::area(body,selection,cx)
}

/// Resolve an icon from the active Papirus theme, including user-installed overrides.
fn papirus_icon(path: &Path, is_dir: bool) -> String {
    if is_dir {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("").to_lowercase();
        let themed = match name.as_str() {
            "documents" => "folder-documents",
            "downloads" => "folder-download",
            "pictures" | "images" => "folder-pictures",
            "music" => "folder-music",
            "videos" => "folder-videos",
            "public" => "folder-publicshare",
            "templates" => "folder-templates",
            _ => "folder",
        };
        return find_papirus_icon("places", themed)
            .or_else(|| find_papirus_icon("places", "folder"))
            .unwrap_or_default();
    }

    let extension = path.extension().and_then(|extension| extension.to_str()).unwrap_or("").to_lowercase();
    let icon = match extension.as_str() {
        "json" => "application-json",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => "image-x-generic",
        "mp3" | "wav" | "flac" | "ogg" => "audio-x-generic",
        "mp4" | "mkv" | "mov" | "webm" => "video-x-generic",
        _ => "text-x-generic",
    };
    find_papirus_icon("mimetypes", icon)
        .or_else(|| find_papirus_icon("mimetypes", "text-x-generic"))
        .unwrap_or_default()
}

/// Resolve system theme names and all XDG icon directories once per process.
/// User icon directories come first so local theme overrides beat packaged ones.
fn find_papirus_icon(category: &str, name: &str) -> Option<String> {
    static THEME_DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    let theme_dirs = THEME_DIRS.get_or_init(resolve_papirus_theme_dirs);
    let sizes = ["128x128", "96x96", "84x84", "64x64", "48x48", "42x42", "32x32", "24x24", "22x22", "16x16", "scalable"];

    // First honor the selected theme's user overrides at every size, then its
    // packaged assets. This is how an icon theme's per-user customizations work.
    for theme_dir in theme_dirs {
        for size in sizes {
            let icon = theme_dir.join(size).join(category).join(format!("{name}.svg"));
            if icon.is_file() { return Some(icon.to_string_lossy().into_owned()); }
        }
    }
    None
}

fn resolve_papirus_theme_dirs() -> Vec<PathBuf> {
    let mut selected_themes = Vec::<String>::new();
    if let Some(theme) = std::env::var("GTK_ICON_THEME").ok().filter(|theme| theme.starts_with("Papirus")) {
        selected_themes.push(theme);
    }

    // Read the desktop's current value. GTK settings files can be stale when
    // the desktop environment keeps its live preference in GSettings.
    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
    {
        if output.status.success() {
            let theme = String::from_utf8_lossy(&output.stdout).trim().trim_matches('\'').to_owned();
            if theme.starts_with("Papirus") && !selected_themes.contains(&theme) {
                selected_themes.push(theme);
            }
        }
    }

    if let Some(home) = std::env::var_os("HOME") {
        for settings in [
            PathBuf::from(&home).join(".config/gtk-4.0/settings.ini"),
            PathBuf::from(&home).join(".config/gtk-3.0/settings.ini"),
        ] {
            if let Ok(contents) = fs::read_to_string(settings) {
                if let Some(theme) = contents.lines().find_map(|line| {
                    line.trim().strip_prefix("gtk-icon-theme-name=")
                        .map(|name| name.trim().trim_matches('"').to_owned())
                }).filter(|theme| theme.starts_with("Papirus")) {
                    if !selected_themes.contains(&theme) { selected_themes.push(theme); }
                    break;
                }
            }
        }
    }

    for fallback in ["Papirus-Dark", "Papirus"] {
        if !selected_themes.iter().any(|theme| theme == fallback) {
            selected_themes.push(fallback.to_owned());
        }
    }

    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut icon_data_dirs = Vec::new();
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        icon_data_dirs.push(PathBuf::from(data_home).join("icons"));
    } else if let Some(home) = &home {
        icon_data_dirs.push(home.join(".local/share/icons"));
    }
    if let Some(home) = &home { icon_data_dirs.push(home.join(".icons")); }
    let data_dirs = std::env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    icon_data_dirs.extend(std::env::split_paths(&data_dirs).map(|dir| dir.join("icons")));

    selected_themes.iter().flat_map(|theme| {
        icon_data_dirs.iter().map(move |icons_dir| icons_dir.join(theme))
    }).filter(|theme_dir| theme_dir.is_dir()).collect()
}

fn format_file_size(path: &Path) -> String {
    match fs::metadata(path) {
        Ok(metadata) => {
            let bytes = metadata.len();
            if bytes < 1024 { format!("{bytes} B") }
            else if bytes < 1024 * 1024 { format!("{:.0} KB", bytes as f64 / 1024.0) }
            else { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
        }
        Err(_) => String::new(),
    }
}
