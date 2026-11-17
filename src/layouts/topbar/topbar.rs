use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;
use crate::windows::MainWindow;
use super::{navigation, search, views, sorting, breadcrumbs, menu};
pub fn topbar(state: &MainWindow, count: usize, cx: &mut Context<MainWindow>) -> AnyElement {
    div().w_full().flex_shrink_0().flex().flex_col().bg(cx.theme().background)
        .border_b_1().border_color(cx.theme().border)
        .child(div().flex().items_center().gap(px(8.)).p(px(14.))
            .child(menu::sidebar_toggle(cx))
            .child(navigation::navigation(state, cx))
            .child(search::search(state))
            .child(views::views(cx))
            .child(sorting::sorting(state, cx))
            .child(super::settings_button::settings_button(cx)))
        .child(breadcrumbs::breadcrumbs(state, count, cx)).into_any_element()
}
