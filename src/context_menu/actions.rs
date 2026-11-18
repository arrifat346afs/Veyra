//! Window commands; filesystem jobs run off the UI thread and report failures in-app.
use super::{
    clipboard,
    dialog::{self, Kind},
    files,
};
use crate::windows::MainWindow;
use gpui_kit::*;
use std::path::PathBuf;
#[derive(Clone, Copy)]
pub enum Command {
    Bookmark,
    NewFolder,
    NewDocument,
    Paste,
    PasteLink,
    SelectAll,
    Hidden,
    Reload,
    Terminal,
    Open,
    ChooseApplication,
    Properties,
    Copy,
    Cut,
    Rename,
    Trash,
    Delete,
}
fn selected(state: &MainWindow) -> Vec<PathBuf> {
    let mut paths: Vec<_> = state.selection.selected.iter().cloned().collect();
    paths.sort();
    paths
}
fn targets(state: &MainWindow) -> Vec<PathBuf> {
    let paths = selected(state);
    if paths.is_empty() {
        vec![state.current_dir.clone()]
    } else {
        paths
    }
}
pub fn run(
    command: Command,
    state: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    if let Some(picker) = &state.picker {
        match command {
            Command::Open => {crate::picker::accept(state,window,cx); return;}
            Command::SelectAll if !picker.request.multiple => return,
            Command::SelectAll | Command::Reload | Command::Hidden | Command::Bookmark => {}
            _ => return,
        }
    }
    if state.file_operation_busy {
        return;
    }
    match command {
        Command::Bookmark => {
            for path in selected(state).into_iter().filter(|path| path.is_dir()) {
                if !bookmark(&path, state, window, cx) { break; }
            }
        }
        Command::NewFolder | Command::NewDocument => {
            let folder = matches!(command, Command::NewFolder);
            dialog::show(
                state,
                Kind::Create {
                    directory: state.current_dir.clone(),
                    folder,
                },
                if folder { "New folder" } else { "New document" },
                format!("Create in {}", state.current_dir.display()),
                Some(if folder {
                    "New folder".into()
                } else {
                    "Untitled.txt".into()
                }),
                window,
                cx,
            );
        }
        Command::Rename => {
            let paths = selected(state);
            if paths.len() != 1 {
                return;
            }
            let path = paths[0].clone();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            dialog::show(
                state,
                Kind::Rename(path),
                "Rename",
                "Enter a new name.",
                Some(name),
                window,
                cx,
            );
        }
        Command::Copy | Command::Cut => {
            let paths = selected(state);
            if paths.is_empty() {
                return;
            }
            clipboard::write(state, &paths, matches!(command, Command::Cut), cx);
            cx.notify();
        }
        Command::Paste | Command::PasteLink => {
            let (paths, cut) = clipboard::read(state, cx);
            if paths.is_empty() {
                return;
            }
            let directory = state.current_dir.clone();
            let link = matches!(command, Command::PasteLink);
            perform(state, window, cx, move || {
                if cut && !link {
                    files::move_to(&paths, &directory)
                } else {
                    files::paste(&paths, &directory, link)
                }
                .map_err(|e| e.to_string())
            });
        }
        Command::Trash => {
            let paths = selected(state);
            if paths.is_empty() {
                return;
            }
            perform(state, window, cx, move || {
                files::trash(&paths).map_err(|e| e.to_string())
            });
        }
        Command::Delete => {
            let paths = selected(state);
            if paths.is_empty() {
                return;
            }
            let detail = format!(
                "Permanently delete {} selected item(s) and their contents? This cannot be undone.",
                paths.len()
            );
            dialog::show(
                state,
                Kind::Delete(paths),
                "Delete permanently?",
                detail,
                None,
                window,
                cx,
            );
        }
        Command::SelectAll => {
            let query = state.search.read(cx).value().to_string();
            let query = if state.preferences.case_sensitive {
                query
            } else {
                query.to_lowercase()
            };
            state.selection.selected = state
                .entries
                .iter()
                .filter(|p| {
                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                    let text = if state.preferences.case_sensitive {
                        name.to_string()
                    } else {
                        name.to_lowercase()
                    };
                    state.picker.as_ref().is_none_or(|picker| picker.request.allows(p) && (if picker.request.directory {p.is_dir()} else {p.is_file()})) && (state.show_hidden || !name.starts_with('.')) && text.contains(&query)
                })
                .cloned()
                .collect();
            cx.notify();
        }
        Command::Hidden => {
            state.show_hidden = !state.show_hidden;
            cx.notify();
        }
        Command::Reload => {
            let path = state.current_dir.clone();
            state.load_directory(path, window, cx);
        }
        Command::Open => {
            let paths = targets(state);
            if paths.len() == 1 && paths[0].is_dir() {
                state.navigate_to(paths[0].clone(), window, cx);
            } else {
                for path in paths {
                    cx.open_with_system(&path);
                }
            }
        }
        Command::ChooseApplication => {
            let paths = targets(state);
            super::application_chooser::open(state,paths,window,cx);
        }
        Command::Terminal => {
            let paths = targets(state);
            let directory = if paths.len() == 1 && paths[0].is_dir() {
                &paths[0]
            } else {
                &state.current_dir
            };
            let result = std::process::Command::new("xdg-terminal-exec")
                .current_dir(directory)
                .spawn()
                .or_else(|_| {
                    std::process::Command::new("x-terminal-emulator")
                        .current_dir(directory)
                        .spawn()
                });
            if let Err(e) = result {
                message(state, "Could not open terminal", e.to_string(), window, cx);
            }
        }
        Command::Properties => {
            let paths = targets(state);
            let detail = paths
                .iter()
                .map(|path| match path.symlink_metadata() {
                    Ok(m) => format!(
                        "{}\nType: {}\nSize: {} bytes\nRead only: {}",
                        path.display(),
                        if m.file_type().is_symlink() {
                            "Symbolic link"
                        } else if m.is_dir() {
                            "Folder"
                        } else {
                            "File"
                        },
                        m.len(),
                        m.permissions().readonly()
                    ),
                    Err(e) => format!("{}: {e}", path.display()),
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            message(state, "Properties", detail, window, cx);
        }
    }
}
pub fn message(
    state: &mut MainWindow,
    title: &str,
    detail: String,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    dialog::show(state, Kind::Message, title, detail, None, window, cx);
}
pub fn perform(
    state: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
    job: impl FnOnce() -> Result<(), String> + Send + 'static,
) {
    state.file_operation_busy = true;
    cx.notify();
    let executor = cx.background_executor().clone();
    cx.spawn_in(window, async move |state, cx| {
        let result = executor.spawn(async move { job() }).await;
        let _ = state.update_in(cx, |state, window, cx| {
            state.file_operation_busy = false;
            match result {
                Ok(()) => {
                    state.file_dialog = None;
                    let path = state.current_dir.clone();
                    state.load_directory(path, window, cx);
                }
                Err(error) => {
                    let path = state.current_dir.clone();
                    state.load_directory(path, window, cx);
                    if let Some(dialog) = state.file_dialog.as_mut() {
                        dialog.error = Some(error);
                        if let Some(input) = dialog.input.clone() {
                            input.update(cx, |input, cx| input.focus(window, cx));
                        }
                        cx.notify();
                    } else {
                        message(state, "File operation failed", error, window, cx);
                    }
                }
            }
        });
    })
    .detach();
}

/// Shared by the folder menu and sidebar drop target. Saving a shortcut never
/// moves or copies the folder; failures are shown in the existing error dialog.
pub fn bookmark(path: &std::path::Path, state: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) -> bool {
    match crate::services::bookmarks::add_bookmark(path) {
        Ok(()) => { cx.notify(); true }
        Err(error) => {
            message(state, "Could not add bookmark", error.to_string(), window, cx);
            false
        }
    }
}
