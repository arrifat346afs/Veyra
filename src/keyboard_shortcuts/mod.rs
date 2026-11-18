//! Application keyboard shortcuts. Add new bindings and handlers here.
mod hidden_files;
mod settings;
mod path_location;
mod file_operations;

use gpui_kit::*;
use crate::windows::MainWindow;

/// Capture app shortcuts before child inputs handle their own bindings.
pub fn attach(root: Div, focus: &FocusHandle, cx: &mut Context<MainWindow>) -> Stateful<Div> {
    root.id("file-manager").track_focus(focus)
        .capture_key_down(cx.listener(|state,event,window,cx| {
            if file_operations::handle(state,event,window,cx) {return;}
            if !settings::handle(state,event,window,cx) && !path_location::handle(state,event,window,cx) && !state.settings_open { hidden_files::handle(state,event,window,cx); }
        }))
}
