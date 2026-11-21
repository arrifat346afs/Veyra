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
    RenameBookmark(PathBuf),
    CreateLink(PathBuf),
    CreateArchive { sources: Vec<PathBuf>, directory: PathBuf, format: crate::services::archives::Format },
    ExtractArchive { archive: PathBuf },
    Delete(Vec<PathBuf>),
    EmptyTrash,
    Conflicts { sources: Vec<PathBuf>, directory: PathBuf, cut: bool, link: bool },
    LinkConflict { source: PathBuf, destination: PathBuf },
    RenameConflict { source: PathBuf, destination: PathBuf },
    Message,
    PickerOverwrite(Vec<PathBuf>),
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
    let placeholder = match &kind {
        Kind::CreateArchive { .. } => "Archive name",
        Kind::ExtractArchive { .. } => "New destination folder path",
        _ => "Name",
    };
    let input = name.map(|name| {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
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
    if let Kind::RenameBookmark(path) = kind {
        let name = name.trim().to_string();
        if name.is_empty() || name.chars().any(char::is_control) {
            state.file_dialog.as_mut().unwrap().error = Some("Enter a bookmark name without control characters.".into());
        } else if let Err(error) = crate::services::bookmarks::rename_bookmark(&path, name) {
            state.file_dialog.as_mut().unwrap().error = Some(error.to_string());
        } else {
            close(state, window, cx);
        }
        cx.notify();
        return;
    }
    if let Kind::PickerOverwrite(paths) = &kind {
        if let Some(picker) = &state.picker {
            if let Err(error) = crate::picker::finish(&picker.request,paths,cx) {
                state.file_dialog.as_mut().unwrap().error=Some(error); cx.notify();
            }
        }
        return;
    }
    if matches!(kind, Kind::Message) {
        close(state, window, cx);
        return;
    }
    if let Kind::ExtractArchive { archive } = &kind {
        let typed = PathBuf::from(name.trim());
        let destination = if typed.is_absolute() { typed } else { archive.parent().unwrap_or_else(|| std::path::Path::new(".")).join(typed) };
        let parent = destination.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or_else(|| std::path::Path::new("."));
        if destination.as_os_str().is_empty() || destination.symlink_metadata().is_ok() || !parent.is_dir() {
            state.file_dialog.as_mut().unwrap().error = Some("Choose a new folder path whose parent already exists.".into());
            cx.notify();
            return;
        }
        let archive = archive.clone();
        super::actions::perform_progress_recorded(state, window, cx, move |progress| {
            progress(0, 1, &archive);
            crate::services::archives::extract(&archive, &destination).map(|()| {
                progress(1, 1, &destination);
                Some(super::files::HistoryRecord {
                    undo: super::files::HistoryAction::Remove(vec![destination.clone()]),
                    redo: super::files::HistoryAction::Extract { archive, destination },
                })
            })
        });
        return;
    }
    if let Kind::CreateArchive { sources, directory, format } = &kind {
        let filename = crate::services::archives::with_extension(name.trim(), *format);
        if let Err(error) = super::files::validate_name(&filename) {
            state.file_dialog.as_mut().unwrap().error = Some(error);
            cx.notify();
            return;
        }
        let destination = directory.join(filename);
        let sources = sources.clone();
        let format = *format;
        super::actions::perform_progress_recorded(state, window, cx, move |progress| {
            progress(0, 1, &destination);
            crate::services::archives::create(&sources, &destination, format).map(|()| {
                progress(1, 1, &destination);
                Some(super::files::HistoryRecord {
                    undo: super::files::HistoryAction::Remove(vec![destination.clone()]),
                    redo: super::files::HistoryAction::CreateArchive { sources, destination, format },
                })
            })
        });
        return;
    }
    if matches!(kind, Kind::Create { .. } | Kind::Rename(_) | Kind::CreateLink(_)) {
        if let Err(error) = super::files::validate_name(&name) {
            state.file_dialog.as_mut().unwrap().error = Some(error);
            cx.notify();
            return;
        }
    }
    if let Kind::CreateLink(source) = &kind {
        let destination = source.parent().unwrap_or_else(|| std::path::Path::new(".")).join(&name);
        if destination.symlink_metadata().is_ok() {
            if let Some(dialog) = state.file_dialog.as_mut() {
                dialog.kind = Kind::LinkConflict { source: source.clone(), destination };
                dialog.title = "File conflict".into();
                dialog.detail = "A link with that name already exists. Choose how to handle the conflict.".into();
            }
            cx.notify();
            return;
        }
    }
    if let Kind::Rename(source) = &kind {
        let destination = source.with_file_name(&name);
        if source != &destination && destination.symlink_metadata().is_ok() {
            if let Some(dialog) = state.file_dialog.as_mut() {
                dialog.kind = Kind::RenameConflict { source: source.clone(), destination };
                dialog.title = "File conflict".into();
                dialog.detail = "A file with that name already exists. Choose how to handle the conflict.".into();
            }
            cx.notify();
            return;
        }
    }
    if let Kind::Conflicts { sources, directory, cut, link } = kind {
        super::actions::resolve_conflict_choice(state, sources, directory, cut, link, super::files::ConflictPolicy::Replace, window, cx);
        return;
    }
    if let Kind::LinkConflict { source, destination } = kind {
        super::actions::resolve_link_conflict_choice(state, source, destination, super::files::ConflictPolicy::Replace, window, cx);
        return;
    }
    if let Kind::RenameConflict { source, destination } = kind {
        super::actions::resolve_rename_conflict_choice(state, source, destination, super::files::ConflictPolicy::Replace, window, cx);
        return;
    }
    super::actions::perform_progress_recorded(state, window, cx, move |progress| {
        match kind {
            Kind::Create { directory, folder } => {
                let path = directory.join(name);
                progress(0, 1, &path);
                super::files::create(&path, folder).map(|()| Some(super::files::HistoryRecord {
                    undo: super::files::HistoryAction::Remove(vec![path.clone()]),
                    redo: super::files::HistoryAction::Create { path, folder },
                })).map(|record| { progress(1, 1, &directory); record })
            }
            Kind::Rename(source) => {
                let destination = source.with_file_name(name);
                progress(0, 1, &source);
                let result = super::files::rename(&source, &destination).map(|()| Some(super::files::HistoryRecord {
                    undo: super::files::HistoryAction::Move(vec![(destination.clone(), source.clone())]),
                    redo: super::files::HistoryAction::Move(vec![(source, destination.clone())]),
                }));
                progress(1, 1, &destination);
                result
            }
            Kind::RenameBookmark(_) => Ok(None),
            Kind::CreateLink(source) => {
                let destination = source.parent().unwrap_or_else(|| std::path::Path::new(".")).join(name);
                progress(0, 1, &source);
                let result = super::files::create_link(&source, &destination).map(|()| Some(super::files::HistoryRecord {
                    undo: super::files::HistoryAction::Remove(vec![destination.clone()]),
                    redo: super::files::HistoryAction::Link(vec![(source, destination.clone())]),
                }));
                progress(1, 1, &destination);
                result
            }
            Kind::CreateArchive { .. } | Kind::ExtractArchive { .. } => Ok(None),
            Kind::Delete(paths) => super::files::delete_with_progress(&paths, progress).map(|()| None),
            Kind::EmptyTrash => super::files::empty_trash_with_progress(progress).map(|()| None),
            Kind::Conflicts { .. } => Ok(None),
            Kind::LinkConflict { .. } => Ok(None),
            Kind::RenameConflict { .. } => Ok(None),
            Kind::Message | Kind::PickerOverwrite(_) => Ok(None),
        }
        .map_err(|e| e.to_string())
    });
}

fn resolve_conflict_button(
    state: &mut MainWindow,
    policy: super::files::ConflictPolicy,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let Some(kind) = state.file_dialog.as_ref().map(|dialog| dialog.kind.clone()) else { return; };
    match kind {
        Kind::Conflicts { sources, directory, cut, link } => super::actions::resolve_conflict_choice(state, sources, directory, cut, link, policy, window, cx),
        Kind::LinkConflict { source, destination } => super::actions::resolve_link_conflict_choice(state, source, destination, policy, window, cx),
        Kind::RenameConflict { source, destination } => super::actions::resolve_rename_conflict_choice(state, source, destination, policy, window, cx),
        _ => {}
    }
}
pub fn view(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let dialog = state.file_dialog.as_ref().unwrap();
    let palette = cx.theme().clone();
    let busy = state.file_operation_busy;
    let label = match dialog.kind {
        Kind::Create { folder: true, .. } => "Create folder",
        Kind::Create { .. } => "Create file",
        Kind::Rename(_) | Kind::RenameBookmark(_) => "Rename",
        Kind::CreateLink(_) => "Create link",
        Kind::CreateArchive { .. } => "Create Archive",
        Kind::ExtractArchive { .. } => "Extract",
        Kind::Delete(_) => "Delete permanently",
        Kind::EmptyTrash => "Empty Trash",
        Kind::Conflicts { .. } => "Replace",
        Kind::LinkConflict { .. } => "Replace",
        Kind::RenameConflict { .. } => "Replace",
        Kind::Message => "Close",
        Kind::PickerOverwrite(_) => "Replace",
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
                .rounded(cx.theme().radius)
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
                        .when(matches!(dialog.kind, Kind::Conflicts { .. } | Kind::LinkConflict { .. } | Kind::RenameConflict { .. }), |row| row
                            .child(Button::new("file-dialog-skip").label("Skip").disabled(busy)
                                .on_click(cx.listener(|s, _, w, c| resolve_conflict_button(s, super::files::ConflictPolicy::Skip, w, c))))
                            .child(Button::new("file-dialog-keep-both").label("Keep Both").disabled(busy)
                                .on_click(cx.listener(|s, _, w, c| resolve_conflict_button(s, super::files::ConflictPolicy::KeepBoth, w, c)))))
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
