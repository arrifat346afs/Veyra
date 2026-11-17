use gpui_kit::*;
use gpui_kit_assets::IconName;
use crate::windows::MainWindow;
use super::button::button;
pub fn sidebar_toggle(foreground_alpha: f32, cx: &mut Context<MainWindow>) -> AnyElement {
    button("sidebar-toggle", IconName::Menu, "", foreground_alpha, cx)
        .on_click(cx.listener(|s, _, _, cx| {s.show_sidebar = !s.show_sidebar; cx.notify();})).into_any_element()
}
