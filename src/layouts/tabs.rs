//! Tab strip, shown only when a window has multiple file-browser tabs.
use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;
use crate::windows::MainWindow;

pub fn view(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    div().id("tab-strip").w_full().h(px(42.)).flex_shrink_0().flex().items_center().justify_between()
        .pl(px(12.)).pr(px(18.)).border_b_1().border_color(cx.theme().border.opacity(0.55))
        .child(div().h_full().flex().items_stretch().gap(px(4.)).children(state.tabs.locations().iter().enumerate().map(|(index, path)| {
            let label = path.file_name().unwrap_or(path.as_os_str()).to_string_lossy().to_string();
            let active = index == state.tabs.active_index();
            div().id(format!("tab-{index}")).h_full().px(px(10.)).flex().items_center().gap(px(8.))
                .border_b_2().border_color(if active {cx.theme().primary} else {cx.theme().transparent})
                .cursor_pointer().text_size(px(12.))
                .text_color(if active {cx.theme().foreground} else {cx.theme().muted_foreground})
                .hover(|el| el.text_color(cx.theme().foreground))
                .child(label)
                .child(div().id(format!("close-tab-{index}")).px(px(3.)).rounded(cx.theme().radius)
                    .text_color(cx.theme().muted_foreground).child("×")
                    .hover(|el| el.bg(cx.theme().primary.opacity(0.12)).text_color(cx.theme().foreground))
                    .on_click(cx.listener(move |s, _, window, cx| {
                        cx.stop_propagation();
                        s.close_tab_at(index, window, cx);
                    })))
                .on_click(cx.listener(move |s, _, window, cx| s.switch_tab(index, window, cx)))
        })))
        .child(div().id("new-tab").size(px(30.)).rounded(cx.theme().radius).flex().items_center().justify_center()
            .cursor_pointer().text_size(px(17.)).text_color(cx.theme().muted_foreground).child("+")
            .hover(|el| el.bg(cx.theme().primary.opacity(0.12)).text_color(cx.theme().foreground))
            .on_click(cx.listener(|s, _, window, cx| s.new_tab(window, cx))))
        .into_any_element()
}
