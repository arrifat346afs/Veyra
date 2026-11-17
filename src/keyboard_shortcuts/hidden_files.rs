use gpui_kit::*;
use crate::windows::MainWindow;

/// Ctrl+H toggles dotfiles throughout the main window, including while searching.
pub(super) fn handle(state: &mut MainWindow, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<MainWindow>) {
    let modifiers = Modifiers { control: true, ..Default::default() };
    if event.keystroke.key == "h" && event.keystroke.modifiers == modifiers {
        // Holding the shortcut should not repeatedly flip visibility.
        if !event.is_held {
            state.show_hidden = !state.show_hidden;
            cx.notify();
        }
        cx.stop_propagation();
    }
}
