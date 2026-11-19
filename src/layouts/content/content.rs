//! Main directory content layout: breadcrumb, file grid, and file icons.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::Icon;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::prelude::*;
use gpui_kit_assets::IconName;

use crate::windows::MainWindow;

/// Render the active path and its file/folder entries in an icon grid.
pub fn content(
    _current_dir: &Path,
    entries: &[PathBuf],
    error: Option<&str>,
    list_view: bool,
    column_view: bool,
    column_paths: &[PathBuf],
    show_hidden: bool,
    search_query: &str,
    preferences: &crate::settings::preferences::Preferences,
    selection: &crate::selection::Selection,
    thumbnail_generation: u64,
    picker: bool,
    cx: &mut Context<MainWindow>,
) -> AnyElement {
    if column_view {
        return columns_content(column_paths, entries, error, preferences, selection, thumbnail_generation, picker, show_hidden, search_query, cx);
    }
    let icon_size = preferences.icon_size.clamp(48,192) as f32;
    let spacing = [20.,12.,8.,4.][preferences.density.min(3)];
    let palette = cx.theme().clone();
    let entry_tiles: Vec<_> = entries.iter().map(|entry| {
        let path = entry.clone();
        let is_dir = entry.is_dir();
        let name = entry.file_name().and_then(|name| name.to_str()).unwrap_or("(unnamed)").to_owned();
        let icon = papirus_icon(entry, is_dir);
        let appimage = entry.extension().and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("appimage"));
        let tile = div().id(format!("entry-{}", entry.display())).w(px(icon_size+52.)).h_auto()
            .flex().flex_col().items_center().justify_start().gap(px(8.0)).p(px(10.0))
            .rounded(cx.theme().radius).text_color(palette.foreground).cursor_pointer()
            .when(is_dir, |tile| tile.on_drag(
                crate::components::folder_drag::FolderDrag { path: entry.clone() },
                |drag, _, _, cx| { cx.stop_propagation(); cx.new(|_| drag.clone()) },
            ))
            .when(list_view, |tile| tile.w_full().h_auto().flex_row().items_center().gap(px(12.)).py(px(4.)))
            .child(if is_dir {
                crate::components::folder_icon::view(PathBuf::from(icon), if list_view {32.} else {icon_size}, cx)
            } else {
                crate::thumbnails::preview(entry, PathBuf::from(icon), preferences.show_thumbnails || appimage, if list_view {32.} else {icon_size}, thumbnail_generation)
            })
            .child(div()
                .when(list_view, |label| label.flex_1().min_w(px(0.)).text_left())
                .when(!list_view, |label| label.w_full().text_center())
                .whitespace_normal().text_size(px(preferences.filename_size.clamp(10,24) as f32)).child(name))
            .when(!is_dir && preferences.show_sizes, |tile| tile.child(div().when(list_view, |size| size.w(px(84.)).text_right())
                .text_color(palette.muted_foreground).text_size(px(11.0))
                .child(format_file_size(entry))))
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| { if !(event.modifiers().control || event.modifiers().shift) && event.click_count() == 2 {if is_dir {this.navigate_to(path.clone(), window, cx);} else if this.picker.is_some() {crate::picker::accept(this,window,cx);} else if crate::context_menu::files::is_zip(&path) {this.selection.selected.clear();this.selection.selected.insert(path.clone());crate::context_menu::actions::run(crate::context_menu::actions::Command::Open,this,window,cx);} else {cx.open_with_system(&path);}} }))
        ;
        crate::selection::item(tile, entry.clone(), selection, picker, cx)
    }).collect();

    let body = div().flex_1().min_h(px(0.)).min_w(px(0.)).h_full().flex().flex_col()
        .child(if let Some(error) = error {
            div().px(px(20.0)).py(px(16.0)).text_color(palette.danger_foreground).child(error.to_owned()).into_any_element()
        } else if entries.is_empty() {
            div().px(px(20.0)).py(px(16.0)).text_color(palette.muted_foreground).child("This folder is empty.").into_any_element()
        } else {
            div().id("file-scroll").flex_1().min_h(px(0.)).w_full()
                .overflow_y_scroll().track_scroll(&selection.scroll)
                .vertical_scrollbar(&selection.scroll).child(
                div().w_full().flex().flex_wrap().content_start().gap(px(spacing)).p(px(18.0)).children(entry_tiles)
            ).h_full().into_any_element()
        }).into_any_element();
    crate::selection::area(body,selection,picker,cx)
}

fn columns_content(
    paths: &[PathBuf],
    current_entries: &[PathBuf],
    error: Option<&str>,
    preferences: &crate::settings::preferences::Preferences,
    selection: &crate::selection::Selection,
    thumbnail_generation: u64,
    picker: bool,
    show_hidden: bool,
    query: &str,
    cx: &mut Context<MainWindow>,
) -> AnyElement {
    let palette = cx.theme().clone();
    let columns = paths.iter().enumerate().map(|(column_index, directory)| {
        let mut items = if column_index + 1 == paths.len() {
            current_entries.to_vec()
        } else {
            fs::read_dir(directory).into_iter().flatten().filter_map(Result::ok).map(|item| item.path()).collect()
        };
        items.retain(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            (show_hidden || !name.starts_with('.')) && (if preferences.case_sensitive {name.to_string()} else {name.to_lowercase()}).contains(query)
        });
        items.sort_by(|a,b| {
            let order = a.file_name().cmp(&b.file_name());
            if preferences.folders_first {b.is_dir().cmp(&a.is_dir()).then(order)} else {order}
        });
        let rows = items.into_iter().map(|entry| {
            let path = entry.clone();
            let is_dir = entry.is_dir();
            let name = entry.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let icon = papirus_icon(&entry, is_dir);
            let row = div().id(format!("column-entry-{}",entry.display())).w_full().min_w(px(0.))
                .h_auto().px(px(10.)).py(px(6.)).rounded(cx.theme().radius)
                .flex().items_center().gap(px(9.)).cursor_pointer()
                .child(if is_dir {
                    crate::components::folder_icon::view(PathBuf::from(icon), 20., cx)
                } else {
                    crate::thumbnails::preview(&entry, PathBuf::from(icon), preferences.show_thumbnails, 20., thumbnail_generation)
                })
                .child(div().flex_1().min_w(px(0.)).whitespace_normal().text_size(px(13.)).child(name))
                .when(is_dir, |row| row.child(Icon::new(IconName::ChevronRight).size(px(14.)).text_color(palette.muted_foreground)))
                .on_click(cx.listener(move |state,event: &ClickEvent,window,cx| {
                    if is_dir {
                        state.selection.selected.clear();
                        state.selection.selected.insert(path.clone());
                        state.open_column(path.clone(),column_index,window,cx);
                    } else if event.click_count() == 2 {
                        cx.open_with_system(&path);
                    }
                }));
            crate::selection::item(row, entry, selection, picker, cx)
        });
        div().id(format!("column-{column_index}")).flex_1().min_w(px(220.)).max_w(px(420.))
            .h_full().overflow_y_scroll().px(px(6.)).py(px(6.))
            .border_r_1().border_color(palette.border.opacity(0.55))
            .flex().flex_col().gap(px(2.)).children(rows)
    });
    let body = if let Some(error) = error {
        div().flex_1().px(px(20.)).py(px(16.)).text_color(palette.danger_foreground).child(error.to_owned()).into_any_element()
    } else {
        div().id("column-view").flex_1().min_h(px(0.)).min_w(px(0.)).h_full().flex().overflow_hidden().children(columns).into_any_element()
    };
    crate::selection::area(body,selection,picker,cx)
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

    let filename = path.file_name().and_then(|name| name.to_str()).unwrap_or("").to_lowercase();
    let extension = path.extension().and_then(|extension| extension.to_str()).unwrap_or("").to_lowercase();
    let icon = if filename.ends_with(".tar.zst") || filename.ends_with(".tzst") {
        "application-x-compressed-tar"
    } else if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
        "application-x-compressed-tar"
    } else if filename.ends_with(".tar.bz2") || filename.ends_with(".tbz") || filename.ends_with(".tbz2") {
        "application-x-bzip-compressed-tar"
    } else if filename.ends_with(".tar.xz") || filename.ends_with(".txz") {
        "application-x-lzma-compressed-tar"
    } else { match extension.as_str() {
        "json" => "application-json",
        "zip" => "application-zip",
        "7z" => "application-x-archive",
        "rar" => "application-x-rar",
        "gz" => "application-x-gzip",
        "bz2" => "application-x-bzip-compressed-tar",
        "xz" => "application-x-lzma-compressed-tar",
        "zst" | "tar" => "application-x-archive",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => "image-x-generic",
        "mp3" | "wav" | "flac" | "ogg" => "audio-x-generic",
        "mp4" | "mkv" | "mov" | "webm" => "video-x-generic",
        _ => "text-x-generic",
    }};
    let archive = matches!(extension.as_str(), "zip" | "7z" | "rar" | "gz" | "bz2" | "xz" | "zst" | "tar")
        || filename.ends_with(".tar.gz") || filename.ends_with(".tar.bz2")
        || filename.ends_with(".tar.xz") || filename.ends_with(".tar.zst")
        || filename.ends_with(".tgz") || filename.ends_with(".tbz")
        || filename.ends_with(".tbz2") || filename.ends_with(".txz")
        || filename.ends_with(".tzst");
    find_papirus_icon("mimetypes", icon)
        .or_else(|| archive.then(|| find_papirus_icon("mimetypes", "application-x-archive")).flatten())
        .or_else(|| archive.then(|| find_papirus_icon("mimetypes", "package-x-generic")).flatten())
        .or_else(|| find_papirus_icon("mimetypes", "text-x-generic"))
        .unwrap_or_default()
}

/// Resolve system theme names and all XDG icon directories once per process.
/// User icon directories come first so local theme overrides beat packaged ones.
fn find_papirus_icon(category: &str, name: &str) -> Option<String> {
    static THEME_DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    let theme_dirs = THEME_DIRS.get_or_init(resolve_papirus_theme_dirs);
    let sizes = ["128x128", "96x96", "84x84", "64x64", "48x48", "42x42", "32x32", "24x24", "22x22", "16x16", "scalable", "symbolic"];

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
    if let Some(theme) = std::env::var("GTK_ICON_THEME").ok().filter(|theme| !theme.is_empty()) {
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
            if !theme.is_empty() && !selected_themes.contains(&theme) {
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
                }).filter(|theme| !theme.is_empty()) {
                    if !selected_themes.contains(&theme) { selected_themes.push(theme); }
                    break;
                }
            }
        }
    }

    for fallback in ["Papirus-Dark", "Papirus", "Adwaita", "breeze", "breeze-dark"] {
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
