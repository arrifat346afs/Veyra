use gpui_kit::*;
use gpui_kit_assets::IconName;
use crate::windows::MainWindow;
pub fn settings_button(foreground_alpha: f32, cx: &mut Context<MainWindow>) -> AnyElement {
    super::button::button("settings",IconName::Settings,"",foreground_alpha,cx)
        .on_click(cx.listener(|s,_,_,cx| crate::settings::window::open(s,cx))).into_any_element()
}
