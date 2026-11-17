use gpui_kit::*;
use gpui_kit_assets::IconName;
use crate::windows::MainWindow;
use super::button::button;
pub fn views(foreground_alpha: f32, cx: &mut Context<MainWindow>) -> AnyElement {
    div().flex().gap(px(5.))
        .child(button("grid-view", IconName::LayoutGrid, "", foreground_alpha, cx).on_click(cx.listener(|s, _, _, cx| {s.list_view = false; cx.notify();})))
        .child(button("list-view", IconName::List, "", foreground_alpha, cx).on_click(cx.listener(|s, _, _, cx| {s.list_view = true; cx.notify();})))
        .into_any_element()
}
