use gpui_kit::*;
use crate::windows::MainWindow;

pub(super) fn handle(state: &mut MainWindow, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<MainWindow>) -> bool {
    if event.keystroke.key != "f" || event.keystroke.modifiers != (Modifiers { control: true, ..Default::default() }) { return false; }
    if !event.is_held { state.search.update(cx, |input,cx| input.focus(window,cx)); }
    cx.stop_propagation();
    true
}
