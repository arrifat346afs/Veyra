use gpui_kit::*;
use gpui_kit::component::{ActiveTheme as _, Icon, input::Input};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit_assets::IconName;
use crate::windows::MainWindow;

pub fn search(state: &MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) -> AnyElement {
    let focused = state.search.read(cx).focus_handle(cx).is_focused(window);
    let empty = state.search.read(cx).value().is_empty();
    let search = state.search.clone();
    div().relative().flex_1().min_w(px(100.)).h(px(40.)).px(px(12.))
        .flex().items_center().rounded(cx.theme().radius).border_1()
        .border_color(cx.theme().border.opacity(if focused {0.65} else {0.25}))
        .child(Input::new(&state.search).appearance(false)
            .prefix(Icon::new(IconName::Search).size(px(16.))))
        .when(empty, |field| field.child(
            div().absolute().left(px(40.)).top(px(0.)).bottom(px(0.)).flex().items_center()
                .text_size(px(13.)).text_color(cx.theme().muted_foreground.opacity(if focused {0.8} else {0.35}))
                .on_mouse_down(gpui_kit::MouseButton::Left, move |_, window, cx| {
                    search.update(cx, |input, cx| input.focus(window, cx));
                })
                .child("Search")
        ))
        .into_any_element()
}
