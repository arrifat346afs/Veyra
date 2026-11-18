//! In-app naming, confirmation and information dialogs; no system file picker.
use crate::windows::MainWindow;
use gpui_kit::component::{
    ActiveTheme as _, Disableable,
    button::*,
    input::{Input, InputEvent, InputState},
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::PathBuf;

#[derive(Clone)]
pub enum Kind {
    Create { directory: PathBuf, folder: bool },
    Rename(PathBuf),
    Delete(Vec<PathBuf>),
    Message,
}
pub struct Dialog {
    pub kind: Kind,
    pub title: String,
    pub detail: String,
    pub input: Option<Entity<InputState>>,
    pub error: Option<String>,
}
pub fn show(
    state: &mut MainWindow,
    kind: Kind,
    title: impl Into<String>,
    detail: impl Into<String>,
    name: Option<String>,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let input = name.map(|name| {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        input.update(cx, |input, cx| {
            input.set_value(name, window, cx);
            input.focus(window, cx);
        });
        cx.subscribe_in(
            &input,
            window,
            |state, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    submit(state, window, cx);
                } else {
                    cx.notify();
                }
            },
        )
        .detach();
        input
    });
    state.file_dialog = Some(Dialog {
        kind,
        title: title.into(),
        detail: detail.into(),
        input,
        error: None,
    });
    // Menus restore focus when dismissed; focus the name field afterwards.
    cx.defer_in(window, |state, window, cx| {
        if let Some(input) = state
            .file_dialog
            .as_ref()
            .and_then(|dialog| dialog.input.clone())
        {
            input.update(cx, |input, cx| input.focus(window, cx));
        }
    });
    cx.notify();
}
pub fn close(state: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    if state.file_operation_busy {
        return;
    }
    state.file_dialog = None;
    state.focus.focus(window, cx);
    cx.notify();
}
pub fn submit(state: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    if state.file_operation_busy {
        return;
    }
    let Some(dialog) = &state.file_dialog else {
        return;
    };
    let kind = dialog.kind.clone();
    let name = dialog
        .input
        .as_ref()
        .map(|input| input.read(cx).value().to_string())
        .unwrap_or_default();
    if matches!(kind, Kind::Message) {
        close(state, window, cx);
        return;
    }
    if matches!(kind, Kind::Create { .. } | Kind::Rename(_)) {
        if let Err(error) = super::files::validate_name(&name) {
            state.file_dialog.as_mut().unwrap().error = Some(error);
            cx.notify();
            return;
        }
    }
    super::actions::perform(state, window, cx, move || {
        match kind {
            Kind::Create { directory, folder } => {
                super::files::create(&directory.join(name), folder)
            }
            Kind::Rename(source) => super::files::rename(&source, &source.with_file_name(name)),
            Kind::Delete(paths) => super::files::delete(&paths),
            Kind::Message => Ok(()),
        }
        .map_err(|e| e.to_string())
    });
}
pub fn view(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let dialog = state.file_dialog.as_ref().unwrap();
    let palette = cx.theme().clone();
    let busy = state.file_operation_busy;
    let label = match dialog.kind {
        Kind::Create { folder: true, .. } => "Create folder",
        Kind::Create { .. } => "Create file",
        Kind::Rename(_) => "Rename",
        Kind::Delete(_) => "Delete permanently",
        Kind::Message => "Close",
    };
    div()
        .absolute()
        .inset_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(0x00000088))
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            div()
                .id("file-operation-dialog")
                .w(px(440.))
                .h_auto()
                .flex_shrink_0()
                .max_h(relative(0.88))
                // The scrollbar wrapper fills its parent. Native overflow lets
                // short dialogs keep their natural height and long ones scroll.
                .overflow_y_scroll()
                .max_w(relative(0.94))
                .p(px(22.))
                .rounded(px(12.))
                .border_1()
                .border_color(palette.border)
                .bg(palette.popover)
                .text_color(palette.popover_foreground)
                .shadow_lg()
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(div().text_size(px(20.)).child(dialog.title.clone()))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(palette.muted_foreground)
                        .child(dialog.detail.clone()),
                )
                .children(dialog.input.as_ref().map(Input::new))
                .children(dialog.error.clone().map(|error| {
                    div()
                        .text_size(px(13.))
                        .text_color(palette.danger)
                        .child(error)
                }))
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .when(!matches!(dialog.kind, Kind::Message), |row| {
                            row.child(
                                Button::new("file-dialog-cancel")
                                    .label("Cancel")
                                    .disabled(busy)
                                    .on_click(cx.listener(|s, _, w, c| close(s, w, c))),
                            )
                        })
                        .child(
                            Button::new("file-dialog-submit")
                                .label(if busy { "Working…" } else { label })
                                .primary()
                                .disabled(busy)
                                .on_click(cx.listener(|s, _, w, c| submit(s, w, c))),
                        ),
                ),
        )
        .into_any_element()
}
