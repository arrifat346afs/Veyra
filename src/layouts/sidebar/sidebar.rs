//! The file manager's persistent navigation sidebar.
//!
//! This layout owns the visual organization of built-in places and user
//! bookmarks. It delegates navigation to `MainWindow`, which owns the active
//! directory and refreshes the content pane. Keeping those responsibilities
//! separate lets this file remain the single place to adjust sidebar layout.

use std::path::{Path, PathBuf};

use gpui_kit::{div, px, Context, ExternalPaths, IntoElement, ParentElement, Styled};
use gpui_kit::prelude::*;
use gpui_kit::component::{ActiveTheme as _, Icon};
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
    // Copy the semantic palette before attaching listeners, so the complete
    // sidebar follows the active theme without embedding color values here.
    let palette = cx.theme().clone();

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
        .child(div().px(px(14.0)).h(px(38.)).flex().items_center().text_size(px(13.)).text_color(palette.link).child("Places"))
        .child(location_row("Recent", IconName::Clock3, crate::services::navigation::recent_location(), current_dir, cx))
        .child(location_row("Starred", IconName::Star, crate::services::navigation::starred_location(), current_dir, cx))
        .child(location_row("Home", IconName::House, home.clone(), current_dir, cx))
        .child(location_row("Desktop", IconName::PanelsTopLeft, home.join("Desktop"), current_dir, cx))
        .child(location_row("Documents", IconName::Files, home.join("Documents"), current_dir, cx))
        .child(location_row("Downloads", IconName::Download, home.join("Downloads"), current_dir, cx))
        .child(location_row("Pictures", IconName::Image, home.join("Pictures"), current_dir, cx))
        .child(location_row("Music", IconName::Music, home.join("Music"), current_dir, cx))
        .child(location_row("Videos", IconName::Clapperboard, home.join("Videos"), current_dir, cx))
        .child(location_row("File system", IconName::HardDrive, PathBuf::from("/"), current_dir, cx))
        .child(location_row("Trash", IconName::Trash, PathBuf::from(crate::context_menu::files::TRASH_URI), current_dir, cx))
        .when(!saved_bookmarks.is_empty(), |sidebar| sidebar.child(div().h(px(16.))))
        .children(saved_bookmarks.into_iter().map(|bookmark| {
            // A bookmark has two independent click targets: its label navigates
            // and its trailing × removes it from the app-owned store.
            let target = bookmark.path.clone();
            let drop_destination = bookmark.path.clone();
            let remove_target = bookmark.path.clone();
            let selected = current_dir == &bookmark.path;
            let foreground = palette.foreground;
            div().h(px(42.0)).flex_shrink_0().w_full().flex().items_center().rounded(cx.theme().radius)
                .text_color(foreground)
                .when(selected, |row| row.bg(palette.primary.opacity(0.22)))
                .hover(|style| style.bg(if selected {palette.primary.opacity(0.28)} else {palette.primary.opacity(0.14)}))
                .child(div().id(format!("bookmark-{}", bookmark.path.display())).flex_1().h_full()
                    .flex().items_center().gap(px(12.0)).px(px(14.0)).cursor_pointer()
                    .hover(|style| style.text_color(if selected {foreground} else {palette.primary}))
                    .on_click(cx.listener(move |this, _, window, cx| this.navigate_to(target.clone(), window, cx)))
                    .on_drop(cx.listener(move |state, paths: &ExternalPaths, window, cx| {
                        crate::context_menu::actions::copy_paths_to(paths.paths().to_vec(), drop_destination.clone(), state, window, cx);
                    }))
                    .child(div().w(px(20.0)).text_color(if selected {foreground} else {palette.muted_foreground})
                        .child(Icon::new(IconName::Bookmark).size(px(18.0))))
                    .child(div().text_ellipsis().child(bookmark.name)))
                .child(div().id(format!("remove-{}", bookmark.path.display())).w(px(36.)).h_full()
                    .flex_shrink_0().flex().items_center().justify_center()
                    .text_color(if selected {foreground} else {palette.muted_foreground}).cursor_pointer()
                    .hover(|style| style.text_color(palette.danger_foreground))
                    .child(Icon::new(IconName::X).size(px(16.0)))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        if let Err(error) = bookmarks::remove_bookmark(&remove_target) {
                            eprintln!("Could not remove bookmark: {error}");
                        }
                        cx.notify();
                    })))
        }))
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
        .child(div().w(px(20.0)).text_color(if selected {foreground} else {palette.muted_foreground})
            .child(Icon::new(icon).size(px(18.0))))
        .child(div().text_ellipsis().child(label))
}
