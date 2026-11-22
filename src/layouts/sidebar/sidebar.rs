//! The file manager's persistent navigation sidebar.
//!
//! This layout owns the visual organization of built-in places and user
//! bookmarks. It delegates navigation to `MainWindow`, which owns the active
//! directory and refreshes the content pane. Keeping those responsibilities
//! separate lets this file remain the single place to adjust sidebar layout.

use std::path::{Path, PathBuf};

use gpui_kit::{div, px, Context, ExternalPaths, IntoElement, ParentElement, Styled, WeakEntity};
use gpui_kit::prelude::*;
use gpui_kit::component::{ActiveTheme as _, Icon};
use gpui_kit::component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_kit_assets::IconName;

use crate::{services::bookmarks, windows::MainWindow};

pub const WIDTH: f32 = 270.;

/// Build the complete sidebar for the current window state.
///
/// Built-in places are derived from the user's home directory. Bookmarks come
/// from the file manager's own persistent store, not another manager's live
/// state. Clicking a location asks `MainWindow` to navigate to that path.
pub fn sidebar(current_dir: &Path, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
    let saved_bookmarks = bookmarks::get_bookmarks();
    let mounted_locations: Vec<_> = {
        use gio::prelude::*;
        gio::VolumeMonitor::get().mounts().into_iter().filter_map(|mount| {
            let root = mount.root();
            let path = root.path()?;
            if path == Path::new("/") || path == home { return None; }
            let uri = root.uri().to_string();
            let label = if !uri.starts_with("file://") { format!("Network: {}", mount.name()) }
                else if mount.can_eject() { format!("Removable: {}", mount.name()) }
                else { format!("Drive: {}", mount.name()) };
            Some((label, path))
        }).collect()
    };
    // Copy the semantic palette before attaching listeners, so the complete
    // sidebar follows the active theme without embedding color values here.
    let palette = cx.theme().clone();
    let mut places = vec![
        ("place:recent".into(), "Recent".into(), location_row("Recent", IconName::Clock3, crate::services::navigation::recent_location(), current_dir, cx).into_any_element()),
        ("place:starred".into(), "Starred".into(), location_row("Starred", IconName::Star, crate::services::navigation::starred_location(), current_dir, cx).into_any_element()),
        ("place:home".into(), "Home".into(), location_row("Home", IconName::House, home.clone(), current_dir, cx).into_any_element()),
        ("place:desktop".into(), "Desktop".into(), location_row("Desktop", IconName::PanelsTopLeft, home.join("Desktop"), current_dir, cx).into_any_element()),
        ("place:documents".into(), "Documents".into(), location_row("Documents", IconName::Files, home.join("Documents"), current_dir, cx).into_any_element()),
        ("place:downloads".into(), "Downloads".into(), location_row("Downloads", IconName::Download, home.join("Downloads"), current_dir, cx).into_any_element()),
        ("place:pictures".into(), "Pictures".into(), location_row("Pictures", IconName::Image, home.join("Pictures"), current_dir, cx).into_any_element()),
        ("place:music".into(), "Music".into(), location_row("Music", IconName::Music, home.join("Music"), current_dir, cx).into_any_element()),
        ("place:videos".into(), "Videos".into(), location_row("Videos", IconName::Clapperboard, home.join("Videos"), current_dir, cx).into_any_element()),
        ("place:file-system".into(), "File system".into(), super::filesystem::row(current_dir, cx).into_any_element()),
    ];
    for (label, path) in mounted_locations {
        places.push((format!("mount:{}", path.display()), label.clone(), location_row_owned(label, path, current_dir, cx).into_any_element()));
    }
    places.push(("place:trash".into(), "Trash".into(), location_row("Trash", IconName::Trash, PathBuf::from(crate::context_menu::files::TRASH_URI), current_dir, cx).into_any_element()));
    for bookmark in saved_bookmarks {
        let key = format!("bookmark:{}", bookmark.path.display());
        let label = bookmark.name.clone();
        places.push((key, label, bookmark_row(bookmark, current_dir, cx).into_any_element()));
    }
    let places = super::ordering::rows(places, cx);

    div().id("bookmark-drop-target").w(px(WIDTH)).flex_shrink_0().h_full().flex().flex_col().gap(px(4.0))
        .px(px(12.0)).py(px(12.0))
        .drag_over::<crate::components::folder_drag::FolderDrag>(|style, _, _, cx| {
            style.bg(cx.theme().primary.opacity(0.12))
        })
        .on_drop(cx.listener(|state, drag: &crate::components::folder_drag::FolderDrag, window, cx| {
            if drag.paths.len() == 1 && drag.paths[0].is_dir() {
                crate::context_menu::actions::bookmark(&drag.paths[0], state, window, cx);
            }
        }))
        .child(div().px(px(14.0)).h(px(38.)).flex().items_center().justify_between()
            .text_size(px(13.)).text_color(palette.link).child("Places")
            .child(div().text_size(px(10.)).text_color(palette.muted_foreground).child("Drag to reorder")))
        .children(places)
}

fn bookmark_row(bookmark: bookmarks::Bookmark, current_dir: &Path, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let target = bookmark.path.clone();
    let drop_destination = bookmark.path.clone();
    let remove_target = bookmark.path.clone();
    let hover_group = format!("bookmark-row-{}", bookmark.path.display());
    let selected = current_dir == &bookmark.path;
    let palette = cx.theme().clone();
    let foreground = palette.foreground;
    div().h(px(42.)).flex_shrink_0().w_full().flex().items_center().rounded(cx.theme().radius)
        .group(hover_group.clone()).text_color(foreground)
        .when(selected, |row| row.bg(palette.primary.opacity(0.22)))
        .hover(|style| style.bg(if selected {palette.primary.opacity(0.28)} else {palette.primary.opacity(0.14)}))
        .child(div().id(format!("bookmark-{}", bookmark.path.display())).flex_1().h_full()
            .flex().items_center().gap(px(12.)).px(px(14.)).cursor_pointer()
            .hover(|style| style.text_color(if selected {foreground} else {palette.primary}))
            .on_click(cx.listener(move |this, _, window, cx| this.navigate_to(target.clone(), window, cx)))
            .on_drop(cx.listener(move |state, paths: &ExternalPaths, window, cx| {
                crate::context_menu::actions::copy_paths_to(paths.paths().to_vec(), drop_destination.clone(), state, window, cx);
            }))
            .child(div().w(px(20.)).text_color(if selected {foreground} else {palette.primary})
                .child(crate::components::folder_icon::view_for_folder(PathBuf::new(), &bookmark.path, 18., cx)))
            .child(div().text_ellipsis().child(bookmark.name)))
        .child(div().id(format!("remove-{}", bookmark.path.display())).w(px(36.)).h_full()
            .flex_shrink_0().flex().items_center().justify_center().opacity(0.)
            .group_hover(hover_group, |style| style.opacity(1.))
            .text_color(if selected {foreground} else {palette.muted_foreground}).cursor_pointer()
            .hover(|style| style.text_color(palette.danger_foreground))
            .child(Icon::new(IconName::X).size(px(16.)))
            .on_click(cx.listener(move |_, _, _, cx| {
                if let Err(error) = bookmarks::remove_bookmark(&remove_target) { eprintln!("Could not remove bookmark: {error}"); }
                cx.notify();
            })))
        .context_menu(sidebar_menu(bookmark.path, true, cx.entity().downgrade()))
}

fn location_row_owned(label: String, path: PathBuf, current_dir: &Path, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let external_drop_destination = path.clone();
    let selected = current_dir == path;
    let palette = cx.theme().clone();
    let foreground = palette.foreground;
    let menu_path = path.clone();
    div().id(format!("mount-{}", path.display())).h(px(42.0)).flex_shrink_0().w_full().flex().items_center()
        .gap(px(12.0)).px(px(14.0)).rounded(cx.theme().radius).text_color(foreground).cursor_pointer()
        .when(selected, |row| row.bg(palette.primary.opacity(0.22)))
        .hover(|style| style.bg(if selected {palette.primary.opacity(0.28)} else {palette.primary.opacity(0.14)}))
        .on_click(cx.listener(move |this, _, window, cx| this.navigate_to(path.clone(), window, cx)))
        .on_drop(cx.listener(move |state, paths: &ExternalPaths, window, cx| crate::context_menu::actions::copy_paths_to(paths.paths().to_vec(), external_drop_destination.clone(), state, window, cx)))
        .child(div().w(px(20.0)).text_color(palette.primary).child(Icon::new(IconName::HardDrive).size(px(18.0))))
        .child(div().text_ellipsis().child(label))
        .context_menu(sidebar_menu(menu_path, false, cx.entity().downgrade()))
}

/// Render one built-in location with a shared shape and navigation behavior.
fn location_row(label: &'static str, icon: IconName, path: PathBuf,
                current_dir: &Path, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let external_drop_destination = path.clone();
    let internal_drop_destination = path.clone();
    let is_trash = crate::context_menu::files::is_trash_location(&path);
    let selected = current_dir == path;
    let palette = cx.theme().clone();
    let foreground = palette.foreground;
    let menu_path = path.clone();
    div().id(format!("place-{label}")).h(px(42.0)).flex_shrink_0().w_full().flex().items_center()
        .gap(px(12.0)).px(px(14.0)).rounded(cx.theme().radius)
        .text_color(foreground).cursor_pointer()
        .when(selected, |row| row.bg(palette.primary.opacity(0.22)))
        .hover(|style| style.bg(if selected {palette.primary.opacity(0.28)} else {palette.primary.opacity(0.14)})
            .text_color(if selected {foreground} else {palette.primary}))
        .on_click(cx.listener(move |this, _, window, cx| this.navigate_to(path.clone(), window, cx)))
        .on_drop(cx.listener(move |state, paths: &ExternalPaths, window, cx| {
            if is_trash {
                state.selection.selected = paths.paths().iter().cloned().collect();
                crate::context_menu::actions::run(crate::context_menu::actions::Command::Trash, state, window, cx);
            } else {
                crate::context_menu::actions::copy_paths_to(paths.paths().to_vec(), external_drop_destination.clone(), state, window, cx);
            }
        }))
        .on_drop(cx.listener(move |state, drag: &crate::components::folder_drag::FolderDrag, window, cx| {
            if is_trash {
                state.selection.selected = drag.paths.iter().cloned().collect();
                crate::context_menu::actions::run(crate::context_menu::actions::Command::Trash, state, window, cx);
            } else {
                crate::context_menu::actions::move_paths_to(drag.paths.clone(), internal_drop_destination.clone(), state, window, cx);
            }
        }))
        .child(div().w(px(20.0)).text_color(if selected {foreground} else {palette.primary})
            .child(Icon::new(icon).size(px(18.0))))
        .child(div().text_ellipsis().child(label))
        .context_menu(sidebar_menu(menu_path, false, cx.entity().downgrade()))
}

pub(super) fn sidebar_menu(path: PathBuf, bookmark: bool, owner: WeakEntity<MainWindow>) -> impl Fn(gpui_kit::component::menu::PopupMenu, &mut gpui_kit::Window, &mut Context<gpui_kit::component::menu::PopupMenu>) -> gpui_kit::component::menu::PopupMenu + 'static {
    move |menu, _, _| {
        let tab_owner = owner.clone();
        let tab_path = path.clone();
        let window_owner = owner.clone();
        let window_path = path.clone();
        let mut menu = menu
            .item(PopupMenuItem::element(|_, _| div().h(px(36.)).pr(px(8.)).flex().items_center().child("Open in new tab"))
                .icon(Icon::new(IconName::Folder).mx(px(4.)).my(px(12.)))
                .on_click(move |_, window, cx| {
                    let _ = tab_owner.update(cx, |state, cx| state.open_in_new_tab(tab_path.clone(), window, cx));
                }))
            .item(PopupMenuItem::element(|_, _| div().h(px(36.)).pr(px(8.)).flex().items_center().child("Open in new window"))
                .icon(Icon::new(IconName::AppWindow).mx(px(4.)).my(px(12.)))
                .on_click(move |_, _, cx| {
                    let _ = window_owner.update(cx, |_, cx| crate::windows::MainWindow::open_at(window_path.clone(), cx));
                }));
        if bookmark {
            let rename_owner = owner.clone();
            let rename_path = path.clone();
            let rename_name = bookmarks::get_bookmarks().into_iter().find(|item| item.path == path).map(|item| item.name).unwrap_or_default();
            menu = menu.item(PopupMenuItem::element(|_, _| div().h(px(36.)).pr(px(8.)).flex().items_center().child("Rename bookmark…"))
                .icon(Icon::new(IconName::Pencil).mx(px(4.)).my(px(12.)))
                .on_click(move |_, window, cx| {
                    let _ = rename_owner.update(cx, |state, cx| crate::context_menu::dialog::show(state,
                        crate::context_menu::dialog::Kind::RenameBookmark(rename_path.clone()), "Rename bookmark", "Choose a name for this sidebar bookmark.", Some(rename_name.clone()), window, cx));
                }));
            let remove_owner = owner.clone();
            let remove_path = path.clone();
            menu = menu.item(PopupMenuItem::element(|_, _| div().h(px(36.)).pr(px(8.)).flex().items_center().child("Remove bookmark"))
                .icon(Icon::new(IconName::Bookmark).mx(px(4.)).my(px(12.)))
                .on_click(move |_, _, cx| {
                    if let Err(error) = bookmarks::remove_bookmark(&remove_path) { eprintln!("Could not remove bookmark: {error}"); }
                    let _ = remove_owner.update(cx, |_, cx| cx.notify());
                }));
        }
        menu
    }
}
