//! Standard location-edit shortcuts for the reusable path bar.
use gpui_kit::*;
use crate::windows::MainWindow;
pub fn handle(state: &mut MainWindow,event: &KeyDownEvent,window: &mut Window,cx: &mut Context<MainWindow>) -> bool {
    if state.path_bar.editing && !state.path_bar.suggestions.is_empty() {
        let len = state.path_bar.suggestions.len();
        match event.keystroke.key.as_str() {
            "down" | "up" => {
                if !event.is_held {
                    state.path_bar.highlighted = Some(match (event.keystroke.key.as_str(),state.path_bar.highlighted) {
                        ("down",Some(index)) => (index+1)%len,
                        ("up",Some(index)) => (index+len-1)%len,
                        ("up",None) => len-1,
                        _ => 0,
                    });
                    cx.notify();
                }
                cx.stop_propagation();return true;
            }
            "tab" => {
                if !event.is_held {
                    state.path_bar.accept(state.path_bar.highlighted.unwrap_or(0),window,cx);
                    cx.notify();
                }
                cx.stop_propagation();return true;
            }
            _ => {}
        }
    }
    let edit = event.keystroke.key == "l"
        && event.keystroke.modifiers == Modifiers {control:true,..Default::default()};
    let cancel = state.path_bar.editing && event.keystroke.key == "escape";
    if !(edit || cancel) {return false;}
    if !event.is_held {
        if edit {let current=state.current_dir.clone();state.path_bar.start(&current,window,cx);}
        else {state.path_bar.cancel();state.search.update(cx, |input,cx| input.focus(window,cx));}
        cx.notify();
    }
    cx.stop_propagation();true
}
