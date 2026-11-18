use gpui_kit::*;
use gpui_kit_assets::IconName;
use super::button::button;
use crate::windows::MainWindow;
pub fn navigation(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let back = !state.back_history.is_empty();
    let forward = !state.forward_history.is_empty();
    div().flex().gap(px(3.))
        .child(button("back", IconName::ArrowLeft, "", state.preferences.foreground_alpha(), cx).opacity(if back {1.} else {0.35})
            .on_click(cx.listener(|s, _, window, cx| s.go_back(window,cx))))
        .child(button("forward", IconName::ArrowRight, "", state.preferences.foreground_alpha(), cx).opacity(if forward {1.} else {0.35})
            .on_click(cx.listener(|s, _, window, cx| s.go_forward(window,cx))))
        .into_any_element()
}
