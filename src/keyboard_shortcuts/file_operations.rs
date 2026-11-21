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
    if state.batch_rename.is_some() {
        if event.keystroke.key == "escape" {
            crate::batch_rename::view::close(state, window, cx);
            cx.stop_propagation();
        }
        return true;
    }
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
    if state.properties_dialog.is_some() {
        if event.keystroke.key == "escape" {
            state.properties_dialog = None;
            state.properties_image_details = false;
            state.properties_metadata = None;
            state.properties_metadata_error = None;
            state.properties_folder_size = None;
            state.properties_filesystem = None;
            state.properties_error = None;
            state.properties_metadata_loading = false;
            state.focus.focus(window, cx);
            cx.notify();
            cx.stop_propagation();
        }
        return true;
    }
    if state.path_bar.editing || !state.focus.is_focused(window) {
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
    if m == ctrl || m == ctrl_shift {
        let adjustment = match key {
            "+" | "=" | "equal" | "kp_add" | "add" => Some(16isize),
            "-" | "minus" | "kp_subtract" | "subtract" => Some(-16isize),
            "0" | "kp_0" => Some(0isize),
            _ => None,
        };
        if let Some(adjustment) = adjustment {
            if !event.is_held {
                let size = if adjustment == 0 {
                    112
                } else {
                    (state.preferences.icon_size as isize + adjustment).clamp(48, 192) as usize
                };
                state.preferences.icon_size = size;
                if let Some(slider) = state.settings_sliders.first() {
                    slider.update(cx, |slider, cx| slider.set_value(size as f32, window, cx));
                }
                state.settings_error = state.preferences.save().err();
                cx.notify();
            }
            cx.stop_propagation();
            return true;
        }
    }
    let command = if m == ctrl {
        match key {
            "c" => Some(Command::Copy),
            "x" => Some(Command::Cut),
            "v" => Some(Command::Paste),
            "a" => Some(Command::SelectAll),
            "r" => Some(Command::Reload),
            "z" => Some(Command::Undo),
            "y" => Some(Command::Redo),
            "f2" => Some(Command::BatchRename),
            _ => None,
        }
    } else if m == ctrl_shift {
        match key {
            "n" => Some(Command::NewFolder),
            "z" => Some(Command::Redo),
            "v" => {
                if !event.is_held { state.cycle_view_mode(cx); }
                cx.stop_propagation();
                return true;
            }
            _ => None,
        }
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
            "space" => Some(Command::Preview),
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
