use crate::windows::MainWindow;
use gpui_kit::*;
pub fn handle(
    state: &mut MainWindow,
    event: &KeyDownEvent,
    _window: &mut Window,
    cx: &mut Context<MainWindow>,
) -> bool {
    let open = event.keystroke.key == ","
        && event.keystroke.modifiers
            == Modifiers {
                control: true,
                ..Default::default()
            };
    if open {
        if !event.is_held {
            crate::settings::window::open(state, cx);
        }
        cx.stop_propagation();
    }
    open
}
