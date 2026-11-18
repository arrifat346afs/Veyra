//! Left-aligned breadcrumbs with an editable location field.
use std::path::{Path, PathBuf};
use gpui_kit::*;
use gpui_kit::component::{ActiveTheme as _, Icon, input::Input};
use gpui_kit::prelude::*;
use gpui_kit_assets::IconName;
use crate::windows::MainWindow;

fn left_inset(state: &MainWindow) -> f32 {
    if state.show_sidebar { crate::layouts::sidebar::sidebar::WIDTH } else { 12. }
}

fn location_width(state: &MainWindow, window_width: f32) -> f32 {
    (window_width - left_inset(state) - 12. - 24.).max(0.)
}

pub fn view(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let palette = cx.theme().clone();
    let surface = palette.muted.opacity(state.preferences.foreground_alpha());
    let location = if state.path_bar.editing {
        Input::new(&state.path_bar.input).w_full().into_any_element()
    } else {
        breadcrumbs(state, surface, cx)
    };
    div().w_full().flex().flex_col().gap(px(2.)).px(px(12.)).pt(px(10.)).pb(px(8.)).flex_shrink_0()
        .child(div().w_full().flex().items_center().gap(px(8.))
            .child(div().id("path-bar-surface").flex_1().min_w(px(0.)).flex().items_center()
                .px(px(4.)).py(px(2.)).rounded(cx.theme().radius).bg(surface)
                .child(div().flex_1().min_w(px(0.)).child(location))
                .when(!state.path_bar.editing, |container| container
                    .on_click(cx.listener(|state,event: &ClickEvent,window,cx| {
                        if event.click_count() == 2 {
                            let current = state.current_dir.clone();
                            state.path_bar.start(&current,window,cx);
                            cx.notify();
                        }
                    })))))
        .children(state.path_bar.error.clone().map(|message| div().text_size(px(12.))
            .text_color(palette.danger_foreground).child(message)))
        .into_any_element()
}

fn breadcrumbs(state: &MainWindow, surface: Hsla, cx: &mut Context<MainWindow>) -> AnyElement {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let inside_home = !home.as_os_str().is_empty() && state.current_dir.starts_with(&home);
    let base = if inside_home { home.as_path() } else { Path::new("/") };
    let base_icon = if inside_home { IconName::House } else { IconName::HardDrive };
    let base_label = if inside_home { "Home" } else { "File system" };
    let muted = cx.theme().muted_foreground;
    let mut crumbs: Vec<AnyElement> = Vec::new();
    let base_target = base.to_path_buf();
    crumbs.push(div().id("breadcrumb-base").px(px(8.)).py(px(4.)).rounded(cx.theme().radius)
        .flex().items_center().gap(px(5.)).cursor_pointer()
        .when(state.current_dir == base, |crumb| crumb.bg(surface))
        .hover(|style| style.bg(surface))
        .child(Icon::new(base_icon).size(px(13.))).child(base_label)
        .on_click(cx.listener(move |state,event: &ClickEvent,window,cx| {
            if event.click_count() == 1 {state.navigate_to(base_target.clone(),window,cx);}
        }))
        .into_any_element());
    let mut path = base.to_path_buf();
    if let Ok(relative) = state.current_dir.strip_prefix(base) {
        for (index, part) in relative.components().enumerate() {
            path.push(part.as_os_str());
            let target = path.clone();
            let active = target == state.current_dir;
            crumbs.push(div().text_color(muted).child(Icon::new(IconName::ChevronRight).size(px(12.))).into_any_element());
            crumbs.push(div().id(format!("breadcrumb-{index}")).px(px(8.)).py(px(4.))
                .rounded(cx.theme().radius).cursor_pointer()
                .when(active, |crumb| crumb.bg(surface))
                .hover(|style| style.bg(surface))
                .child(part.as_os_str().to_string_lossy().into_owned())
                .on_click(cx.listener(move |state,event: &ClickEvent,window,cx| {
                    if event.click_count() == 1 {state.navigate_to(target.clone(),window,cx);}
                }))
                .into_any_element());
        }
    }
    div().flex().items_center().min_w(px(0.)).overflow_hidden().gap(px(3.)).children(crumbs).into_any_element()
}

/// Draw after the file pane so completion rows remain above its content.
pub fn suggestions_overlay(state: &MainWindow, window_width: f32, cx: &mut Context<MainWindow>) -> AnyElement {
    let palette = cx.theme().clone();
    div().absolute().top(px(110.)).left(px(left_inset(state) + 12.)).w(px(location_width(state, window_width))).flex()
        .child(div().w_full()
            .when(!state.path_bar.suggestions.is_empty(), |list| list
                .p(px(5.)).rounded(cx.theme().radius)
                .bg(palette.muted)
                .occlude())
            .children(state.path_bar.suggestions.iter().enumerate().map(|(index,path)| {
                let target = path.clone();
                let label = path.file_name().map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                let selected = state.path_bar.highlighted == Some(index);
                div().id(format!("path-suggestion-{index}")).w_full().px(px(12.)).py(px(9.))
                    .flex().items_center().gap(px(9.)).rounded(cx.theme().radius).cursor_pointer()
                    .when(selected, |row| row.bg(palette.primary.opacity(0.23)))
                    .hover(|style| style.bg(palette.list_hover))
                    .child(Icon::new(IconName::Folder).size(px(16.)))
                    .child(label)
                    .on_click(cx.listener(move |state,_,window,cx| {
                        if let Some(index) = state.path_bar.suggestions.iter().position(|path| path == &target) {
                            state.path_bar.accept(index,window,cx);
                            cx.notify();
                        }
                    }))
            })))
        .into_any_element()
}
