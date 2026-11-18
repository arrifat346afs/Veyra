//! File commands apply only when the file pane owns keyboard focus.
use crate::{
    context_menu::{
        actions::{self, Command},
        dialog,
    },
    windows::MainWindow,
};
use gpui_kit::*;
pub(super) fn handle(
    state: &mut MainWindow,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) -> bool {
    if state.file_dialog.is_some() {
        if event.keystroke.key == "escape" {
            dialog::close(state, window, cx);
            cx.stop_propagation();
        }
        if event.keystroke.key == "enter"
            && !event.is_held
            && state
                .file_dialog
                .as_ref()
                .is_some_and(|dialog| dialog.input.is_none())
        {
            dialog::submit(state, window, cx);
            cx.stop_propagation();
        }
        return true;
    }
    if state.settings_open || state.path_bar.editing || !state.focus.is_focused(window) {
        return false;
    }
    let key = event.keystroke.key.as_str();
    let m = event.keystroke.modifiers;
    let ctrl = Modifiers {
        control: true,
        ..Default::default()
    };
    let ctrl_shift = Modifiers {
        control: true,
        shift: true,
        ..Default::default()
    };
    let command = if m == ctrl {
        match key {
            "c" => Some(Command::Copy),
            "x" => Some(Command::Cut),
            "v" => Some(Command::Paste),
            "a" => Some(Command::SelectAll),
            "r" => Some(Command::Reload),
            _ => None,
        }
    } else if m == ctrl_shift && key == "n" {
        Some(Command::NewFolder)
    } else if m
        == (Modifiers {
            shift: true,
            ..Default::default()
        })
        && key == "delete"
    {
        Some(Command::Delete)
    } else if m == Modifiers::default() {
        match key {
            "f2" => Some(Command::Rename),
            "f5" => Some(Command::Reload),
            "delete" => Some(Command::Trash),
            "enter" => Some(Command::Open),
            _ => None,
        }
    } else {
        None
    };
    if let Some(command) = command {
        if !event.is_held {
            actions::run(command, state, window, cx);
        }
        cx.stop_propagation();
        true
    } else {
        false
    }
}
