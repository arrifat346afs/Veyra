use gpui_kit::*;
use gpui_kit_assets::IconName;
use super::button::button;
use crate::windows::MainWindow;
pub fn navigation(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let back = !state.back_history.is_empty();
    let forward = !state.forward_history.is_empty();
    div().flex().gap(px(3.))
        .child(button("back", IconName::ArrowLeft, "", cx).opacity(if back {1.} else {0.35})
            .on_click(cx.listener(|s, _, _, cx| s.go_back(cx))))
        .child(button("forward", IconName::ArrowRight, "", cx).opacity(if forward {1.} else {0.35})
            .on_click(cx.listener(|s, _, _, cx| s.go_forward(cx))))
        .into_any_element()
}
