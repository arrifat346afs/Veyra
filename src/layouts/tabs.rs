//! Tab strip, shown only when a window has multiple file-browser tabs.
use gpui_kit::*;
use gpui_kit::prelude::*;
use gpui_kit::component::ActiveTheme as _;
use crate::windows::MainWindow;

pub fn view(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    div().id("tab-strip").w_full().flex_shrink_0().flex().items_center().gap(px(6.))
        .px(px(12.)).pb(px(8.))
        .children(state.tabs.locations().iter().enumerate().map(|(index, path)| {
            let label = path.file_name().unwrap_or(path.as_os_str()).to_string_lossy().to_string();
            let active = index == state.tabs.active_index();
            div().id(format!("tab-{index}")).px(px(12.)).py(px(7.)).rounded(cx.theme().radius)
                .cursor_pointer().text_size(px(12.))
                .text_color(if active { cx.theme().foreground } else { cx.theme().muted_foreground })
                .when(active, |el| el.bg(cx.theme().primary.opacity(0.20)))
                .hover(|el| el.bg(cx.theme().primary.opacity(0.12)))
                .child(label)
                .on_click(cx.listener(move |s, _, window, cx| s.switch_tab(index, window, cx)))
        }))
        .child(div().id("new-tab").px(px(9.)).py(px(6.)).rounded(cx.theme().radius)
            .cursor_pointer().text_color(cx.theme().muted_foreground).child("+")
            .hover(|el| el.bg(cx.theme().primary.opacity(0.12)))
            .on_click(cx.listener(|s, _, window, cx| s.new_tab(window, cx))))
        .child(div().id("close-tab").px(px(9.)).py(px(6.)).rounded(cx.theme().radius)
            .cursor_pointer().text_color(cx.theme().muted_foreground).child("×")
            .hover(|el| el.bg(cx.theme().primary.opacity(0.12)))
            .on_click(cx.listener(|s, _, window, cx| s.close_tab(window, cx))))
        .into_any_element()
}
