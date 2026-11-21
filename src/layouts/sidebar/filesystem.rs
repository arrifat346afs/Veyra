//! Root filesystem place with a compact disk usage indicator.

use std::path::PathBuf;
use gpui_kit::{div, px, relative, Context, IntoElement, InteractiveElement, ParentElement, Styled};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::component::{ActiveTheme as _, Icon};
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit_assets::IconName;
use crate::{properties::system, windows::MainWindow};

pub fn row(current_dir: &std::path::Path, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let palette = cx.theme().clone();
    let path = PathBuf::from("/");
    let selected = current_dir == path;
    let usage = system::filesystem_stats(&path).map(|(_, total, free)| {
        let used = total.saturating_sub(free.min(total));
        (used, total, if total == 0 {0.} else {used as f32 / total as f32})
    });
    let fraction = usage.map(|(_, _, fraction)| fraction).unwrap_or(0.);
    let detail = usage.map(|(_, _, fraction)| format!("{}% used", (fraction * 100.).round() as u32))
        .unwrap_or_else(|| "Storage unavailable".into());
    let owner = cx.entity().downgrade();
    let destination = path.clone();
    let foreground = palette.foreground;
    div().id("place-file-system").h(px(48.)).flex_shrink_0().w_full().flex().items_center()
        .gap(px(12.)).px(px(14.)).rounded(cx.theme().radius).text_color(foreground).cursor_pointer()
        .when(selected, |row| row.bg(palette.primary.opacity(0.22)))
        .hover(|style| style.bg(if selected {palette.primary.opacity(0.28)} else {palette.primary.opacity(0.14)}))
        .on_click(cx.listener(move |state, _, window, cx| state.navigate_to(path.clone(), window, cx)))
        .child(div().w(px(20.)).text_color(palette.muted_foreground).child(Icon::new(IconName::HardDrive).size(px(18.))))
        .child(div().flex_1().flex().flex_col().justify_center().gap(px(3.))
            .child(div().w_full().flex().items_center().justify_between().child("File system")
                .child(div().text_size(px(10.)).text_color(palette.muted_foreground).child(detail)))
            .child(div().id("filesystem-usage-track").w_full().h(px(4.)).rounded_full().bg(palette.border.opacity(0.65))
                .child(div().h_full().w(relative(fraction)).rounded_full().bg(palette.primary))))
        .context_menu(super::sidebar::sidebar_menu(destination, false, owner))
}
