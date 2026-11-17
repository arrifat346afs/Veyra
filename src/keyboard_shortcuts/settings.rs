use gpui_kit::*;
use crate::windows::MainWindow;
pub fn handle(state: &mut MainWindow,event: &KeyDownEvent,window: &mut Window,cx: &mut Context<MainWindow>) -> bool {
    let open=event.keystroke.key=="," && event.keystroke.modifiers==Modifiers {control:true,..Default::default()};
    let close=state.settings_open && event.keystroke.key=="escape";
    if open || close { if !event.is_held { state.settings_open=open;
        let input = if open {state.settings_search.clone()} else {state.search.clone()};
        input.update(cx, |input,cx| input.focus(window,cx));
        cx.notify(); } cx.stop_propagation(); true } else {false}
}
