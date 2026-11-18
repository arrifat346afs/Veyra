use super::finish;
use crate::windows::MainWindow;
use gpui_kit::{
    component::{ActiveTheme as _, Disableable, button::*, input::Input},
    *,
};

pub fn accept(state: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let Some(picker) = &state.picker else {
        return;
    };
    let name = picker.filename.read(cx).value().to_string();
    match picker
        .request
        .selected_paths(&state.current_dir, &state.selection.selected, &name)
    {
        Ok(paths) => {
            if picker.request.save && paths[0].exists() {
                crate::context_menu::dialog::show(
                    state,
                    crate::context_menu::dialog::Kind::PickerOverwrite(paths.clone()),
                    "Replace existing file?",
                    format!("{} already exists. Replace it?", paths[0].display()),
                    None,
                    window,
                    cx,
                );
                return;
            }
            if let Err(error) = finish(&picker.request, &paths, cx) {
                state.picker.as_mut().unwrap().error = Some(error);
                cx.notify();
            }
        }
        Err(error) => {
            state.picker.as_mut().unwrap().error = Some(error);
            cx.notify();
        }
    }
}

pub fn handle_key(
    state: &mut MainWindow,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) -> bool {
    if state.picker.is_none() || state.file_dialog.is_some() || state.path_bar.editing {
        return false;
    }
    if event.keystroke.key == "escape" {
        cx.quit();
        cx.stop_propagation();
        return true;
    }
    if event.keystroke.key == "enter" && !event.is_held && state.focus.is_focused(window) {
        let paths: Vec<_> = state.selection.selected.iter().cloned().collect();
        if paths.len() == 1
            && paths[0].is_dir()
            && !state.picker.as_ref().unwrap().request.directory
        {
            state.navigate_to(paths[0].clone(), window, cx);
        } else {
            accept(state, window, cx);
        }
        cx.stop_propagation();
        return true;
    }
    false
}

pub fn footer(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let picker = state.picker.as_ref().unwrap();
    let request = &picker.request;
    let name = picker.filename.read(cx).value().to_string();
    let valid = request
        .selected_paths(&state.current_dir, &state.selection.selected, &name)
        .is_ok();
    let label = if request.accept_label.is_empty() {
        if request.save {
            "Save"
        } else if request.directory {
            "Select folder"
        } else {
            "Open"
        }
    } else {
        &request.accept_label
    };
    let mut footer = div()
        .w_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(8.))
        .px(px(12.))
        .py(px(10.));
    if request.save {
        footer = footer.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child("File name")
                .child(Input::new(&picker.filename)),
        );
    }
    if let Some(error) = &picker.error {
        footer = footer.child(div().text_color(cx.theme().danger).child(error.clone()));
    }
    let mut options = div().flex().flex_wrap().gap(px(8.));
    if let Some((name, _)) = request.filters.get(request.filter_index) {
        options = options.child(
            Button::new("picker-filter")
                .label(format!("File type: {name}"))
                .on_click(cx.listener(|state, _, _, cx| {
                    let picker = state.picker.as_mut().unwrap();
                    picker.request.filter_index =
                        (picker.request.filter_index + 1) % picker.request.filters.len();
                    picker.error = None;
                    cx.notify();
                })),
        );
    }
    for (index, (_, name, values, value)) in request.choices.iter().enumerate() {
        let display = values
            .iter()
            .find(|(id, _)| id == value)
            .map(|(_, label)| label.as_str())
            .unwrap_or(value);
        options = options.child(
            Button::new(("picker-choice", index))
                .label(format!("{name}: {display}"))
                .on_click(cx.listener(move |state, _, _, cx| {
                    let (_, _, values, value) =
                        &mut state.picker.as_mut().unwrap().request.choices[index];
                    if values.is_empty() {
                        *value = if value == "true" { "false" } else { "true" }.into();
                    } else {
                        let index = values.iter().position(|(id, _)| id == value).unwrap_or(0);
                        *value = values[(index + 1) % values.len()].0.clone();
                    }
                    cx.notify();
                })),
        );
    }
    footer
        .child(options)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(12.))
                        .text_color(cx.theme().muted_foreground)
                        .child(if request.directory {
                            "Choose a folder".into()
                        } else if request.multiple {
                            format!(
                                "{} selected · Ctrl/Shift-click to select multiple files",
                                state.selection.selected.len()
                            )
                        } else {
                            "Choose a file".into()
                        }),
                )
                .child(
                    Button::new("picker-cancel")
                        .label("Cancel")
                        .on_click(cx.listener(|_, _, _, cx| cx.quit())),
                )
                .child(
                    Button::new("picker-accept")
                        .label(label.to_owned())
                        .primary()
                        .disabled(!valid)
                        .on_click(cx.listener(|s, _, w, c| accept(s, w, c))),
                ),
        )
        .into_any_element()
}
