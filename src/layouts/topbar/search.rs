use gpui_kit::*;
use gpui_kit::component::{ActiveTheme as _, Icon, input::Input};
use gpui_kit_assets::IconName;
use crate::windows::MainWindow;

pub fn search(state: &MainWindow, cx: &App) -> AnyElement {
    div().flex_1().min_w(px(100.)).h(px(40.)).px(px(12.))
        .flex().items_center().rounded(cx.theme().radius).border_1().border_color(cx.theme().border)
        .child(Input::new(&state.search).appearance(false)
            .prefix(Icon::new(IconName::Search).size(px(16.))))
        .into_any_element()
}
