use gpui_kit::*;
use gpui_kit_assets::IconName;
use crate::windows::MainWindow;
pub fn settings_button(cx: &mut Context<MainWindow>) -> AnyElement {
    super::button::button("settings",IconName::Settings,"",cx)
        .on_click(cx.listener(|s,_,window,cx| {s.settings_open=true;s.settings_search.update(cx, |input,cx| input.focus(window,cx));cx.notify();})).into_any_element()
}
