//! Settings modal geometry and backdrop, independent of the settings pages.
use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;
use crate::windows::MainWindow;

pub fn popup(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let border = cx.theme().border;
    div().absolute().inset_0().size_full().flex().items_center().justify_center()
        .bg(rgba(0x00000088)).occlude()
        .child(div().w(px(880.)).h(px(600.)).max_w(relative(0.94)).max_h(relative(0.88))
            .rounded(px(14.)).border_1().border_color(border).shadow_lg().overflow_hidden()
            .child(super::panel::panel(state, cx)))
        .into_any_element()
}
