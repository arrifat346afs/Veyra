//! The file manager's persistent navigation sidebar.
//!
//! This layout owns the visual organization of built-in places and user
//! bookmarks. It delegates navigation to `MainWindow`, which owns the active
//! directory and refreshes the content pane. Keeping those responsibilities
//! separate lets this file remain the single place to adjust sidebar layout.

use std::path::PathBuf;

use gpui_kit::{div, px, Context, IntoElement, ParentElement, Styled};
use gpui_kit::prelude::*;
use gpui_kit::component::{ActiveTheme as _, Icon};
use gpui_kit_assets::IconName;

use crate::{services::bookmarks, windows::MainWindow};

/// Build the complete sidebar for the current window state.
///
/// Built-in places are derived from the user's home directory. Bookmarks come
/// from the file manager's own persistent store, not another manager's live
/// state. Clicking a location asks `MainWindow` to navigate to that path.
pub fn sidebar(cx: &mut Context<MainWindow>) -> impl IntoElement {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
    let saved_bookmarks = bookmarks::get_bookmarks();
    // Copy the semantic palette before attaching listeners, so the complete
    // sidebar follows the active theme without embedding color values here.
    let palette = cx.theme().clone();

    div().w(px(238.0)).h_full().flex().flex_col().gap(px(3.0))
        .px(px(12.0)).py(px(16.0)).bg(palette.muted)
        .child(div().px(px(12.0)).pb(px(12.0)).text_color(palette.foreground).child("Places"))
        .child(location_row("Home", IconName::House, home.clone(), cx))
        .child(location_row("Desktop", IconName::PanelsTopLeft, home.join("Desktop"), cx))
        .child(location_row("Documents", IconName::Files, home.join("Documents"), cx))
        .child(location_row("Downloads", IconName::Download, home.join("Downloads"), cx))
        .child(location_row("Pictures", IconName::Image, home.join("Pictures"), cx))
        .child(location_row("Music", IconName::Music, home.join("Music"), cx))
        .child(location_row("Videos", IconName::Clapperboard, home.join("Videos"), cx))
        .child(location_row("File system", IconName::HardDrive, PathBuf::from("/"), cx))
        .child(div().flex().items_center().justify_between().px(px(12.0)).pt(px(20.0)).pb(px(8.0))
            .text_color(palette.foreground).child("Bookmarks")
            .child(div().id("add-bookmark").flex().items_center().gap(px(4.0)).text_color(palette.link)
                .child(Icon::new(IconName::Plus).size(px(14.0))).child("Add current")
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Err(error) = bookmarks::add_bookmark(&this.current_dir) {
                        // Navigation state stays valid; the file list area is
                        // already the app's place for visible filesystem errors.
                        eprintln!("Could not save bookmark: {error}");
                    }
                    cx.notify();
                }))))
        .children(saved_bookmarks.into_iter().map(|bookmark| {
            // A bookmark has two independent click targets: its label navigates
            // and its trailing × removes it from the app-owned store.
            let target = bookmark.path.clone();
            let remove_target = bookmark.path.clone();
            div().h(px(34.0)).w_full().flex().items_center().rounded(px(5.0))
                .text_color(palette.foreground).hover(|style| style.bg(palette.list_hover))
                .child(div().id(format!("bookmark-{}", bookmark.path.display())).flex_1().h_full()
                    .flex().items_center().gap(px(10.0)).px(px(12.0))
                    .on_click(cx.listener(move |this, _, _, cx| this.navigate_to(target.clone(), cx)))
                    .child(div().w(px(18.0)).text_color(palette.muted_foreground)
                        .child(Icon::new(IconName::Bookmark).size(px(14.0))))
                    .child(div().text_ellipsis().child(bookmark.name)))
                .child(div().id(format!("remove-{}", bookmark.path.display())).px(px(10.0))
                    .text_color(palette.muted_foreground).child(Icon::new(IconName::X).size(px(14.0)))
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
                cx: &mut Context<MainWindow>) -> impl IntoElement {
    div().id(format!("place-{label}")).h(px(34.0)).w_full().flex().items_center()
        .gap(px(10.0)).px(px(12.0)).rounded(px(5.0))
        .text_color(cx.theme().foreground).hover(|style| style.bg(cx.theme().list_hover))
        .on_click(cx.listener(move |this, _, _, cx| this.navigate_to(path.clone(), cx)))
        .child(div().w(px(18.0)).text_color(cx.theme().muted_foreground)
            .child(Icon::new(icon).size(px(16.0))))
        .child(div().text_ellipsis().child(label))
}
