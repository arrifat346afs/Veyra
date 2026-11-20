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
    ToggleStar,
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
    Preview,
    Properties,
    ExtractZip,
    Copy,
    CopyPath,
    SetFolderIcon,
    ResetFolderIcon,
    Cut,
    Rename,
    CreateLink,
    BatchRename,
    Trash,
    Delete,
    Restore,
    EmptyTrash,
    Undo,
    Redo,
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
            Command::SelectAll | Command::Reload | Command::Hidden | Command::Bookmark | Command::ToggleStar | Command::Preview => {}
            _ => return,
        }
    }
    if state.file_operation_busy {
        return;
    }
    match command {
        Command::Undo => {
            if let Some(record) = state.undo_history.pop() {
                perform_history(state, window, cx, record, true);
            }
        }
        Command::Redo => {
            if let Some(record) = state.redo_history.pop() {
                perform_history(state, window, cx, record, false);
            }
        }
        Command::Bookmark => {
            for path in selected(state).into_iter().filter(|path| path.is_dir()) {
                if !bookmark(&path, state, window, cx) { break; }
            }
        }
        Command::SetFolderIcon => {
            let paths = selected(state);
            if paths.len() != 1 || !paths[0].is_dir() { return; }
            super::folder_icon_picker::open(paths[0].clone(), window, cx);
        }
        Command::ResetFolderIcon => {
            if let Some(folder) = selected(state).into_iter().find(|path| path.is_dir()) {
                if let Err(error) = crate::services::folder_icons::remove(&folder) {
                    message(state, "Could not reset folder icon", error.to_string(), window, cx);
                } else { cx.notify(); }
            }
        }
        Command::ToggleStar => {
            for path in selected(state) {
                if let Err(error) = crate::services::navigation::toggle_star(&path) {
                    message(state, "Could not update Starred", error.to_string(), window, cx);
                    return;
                }
            }
            if state.current_dir == crate::services::navigation::starred_location() {
                state.entries = crate::services::navigation::starred().into_iter().filter(|p| p.exists()).collect();
            }
            cx.notify();
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
        Command::CreateLink => {
            let paths = selected(state);
            if paths.len() != 1 { return; }
            let source = paths[0].clone();
            let filename = source.file_name().unwrap_or_default().to_string_lossy();
            let suggested = if let Some(extension) = source.extension() {
                format!("{} (link).{}", source.file_stem().unwrap_or_default().to_string_lossy(), extension.to_string_lossy())
            } else {
                format!("{filename} (link)")
            };
            dialog::show(state, Kind::CreateLink(source), "Create symbolic link", "Choose the link name.", Some(suggested), window, cx);
        }
        Command::BatchRename => {
            let paths = selected(state);
            if paths.len() < 2 {
                return;
            }
            crate::batch_rename::open(state, window, cx);
        }
        Command::Copy | Command::Cut => {
            let paths = selected(state);
            if paths.is_empty() {
                return;
            }
            clipboard::write(state, &paths, matches!(command, Command::Cut), cx);
            cx.notify();
        }
        Command::CopyPath => {
            let paths = selected(state);
            if paths.is_empty() { return; }
            state.cut_clipboard = None;
            cx.write_to_clipboard(ClipboardItem::new_string(paths.iter()
                .map(|path| path.to_string_lossy()).collect::<Vec<_>>().join("\n")));
        }
        Command::Paste | Command::PasteLink => {
            let (paths, cut) = clipboard::read(state, cx);
            if paths.is_empty() {
                return;
            }
            let directory = state.current_dir.clone();
            let link = matches!(command, Command::PasteLink);
            if files::has_conflicts(&paths, &directory) {
                dialog::show(state, Kind::Conflicts { sources: paths, directory, cut: cut && !link, link }, "File conflict", "A destination with the same name already exists. Choose how to handle conflicts.", None, window, cx);
            } else {
                transfer_paths_to(paths, directory, cut && !link, link, files::ConflictPolicy::Replace, state, window, cx);
            }
        }
        Command::Trash => {
            let paths = selected(state);
            if paths.is_empty() {
                return;
            }
            if paths.iter().all(|path| path.to_string_lossy().starts_with("trash://")) {
                perform_progress_recorded(state, window, cx, move |progress| files::delete_from_trash_with_progress(&paths, progress).map(|()| None).map_err(|e| e.to_string()));
                return;
            }
            perform_progress_recorded(state, window, cx, move |progress| {
                files::trash_with_progress(&paths, progress).map(|()| Some(files::HistoryRecord {
                    undo: files::HistoryAction::Restore(paths.clone()),
                    redo: files::HistoryAction::Trash(paths),
                })).map_err(|e| e.to_string())
            });
        }
        Command::Restore => {
            let paths = selected(state);
            if paths.is_empty() { return; }
            perform_progress_recorded(state, window, cx, move |progress| {
                let originals = files::originals_for_trash(&paths).map_err(|e| e.to_string())?;
                files::restore_originals_with_progress(&originals, progress).map_err(|e| e.to_string())?;
                Ok(Some(files::HistoryRecord {
                    undo: files::HistoryAction::Trash(originals.clone()),
                    redo: files::HistoryAction::Restore(originals),
                }))
            });
        }
        Command::EmptyTrash => {
            dialog::show(state, Kind::EmptyTrash, "Empty Trash?", "All items in Trash will be permanently deleted.", None, window, cx);
        }
        Command::ExtractZip => {
            let paths = selected(state);
            if paths.len() != 1 || !super::files::is_zip(&paths[0]) { return; }
            let archive = paths[0].clone();
            let destination = super::files::zip_destination(&archive);
            perform_progress_recorded(state, window, cx, move |progress| {
                progress(0, 1, &archive);
                super::files::extract_zip_to_folder(&archive, &destination).map(|()| {
                    progress(1, 1, &archive);
                    Some(files::HistoryRecord {
                        undo: files::HistoryAction::Remove(vec![destination.clone()]),
                        redo: files::HistoryAction::Extract { archive, destination },
                    })
                })
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
            } else if paths.len() == 1 && super::files::is_zip(&paths[0]) {
                let archive = paths[0].clone();
                let destination = super::files::zip_destination(&archive);
                perform_and_navigate(state, window, cx, destination.clone(), archive.clone(), move || {
                    super::files::extract_zip_to_folder(&archive, &destination)
                });
            } else {
                for path in paths {
                    crate::services::navigation::record_recent(&path);
                    cx.open_with_system(&path);
                }
            }
        }
        Command::Preview => crate::windows::quick_preview::toggle(state,window,cx),
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
            if let Some(path) = paths.first() { crate::properties::open(path.clone(), state, window, cx); }
        }
    }
}

fn perform_history(
    state: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
    record: files::HistoryRecord,
    undoing: bool,
) {
    state.file_operation_busy = true;
    state.file_operation_progress = None;
    cx.notify();
    let executor = cx.background_executor().clone();
    cx.spawn_in(window, async move |state, cx| {
        let action = if undoing { record.undo.clone() } else { record.redo.clone() };
        let result = executor.spawn(async move { files::apply_history_action(&action).map_err(|e| e.to_string()) }).await;
        let _ = state.update_in(cx, |state, window, cx| {
            state.file_operation_busy = false;
            state.file_operation_progress = None;
            match result {
                Ok(()) => {
                    if undoing { state.redo_history.push(record); } else { state.undo_history.push(record); }
                    state.reconcile_entries_after_operation();
                    cx.notify();
                }
                Err(error) => {
                    if undoing { state.undo_history.push(record); } else { state.redo_history.push(record); }
                    message(state, "Undo/redo failed", error, window, cx);
                }
            }
        });
    }).detach();
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
pub fn perform_progress_recorded(
    state: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
    job: impl FnOnce(&mut dyn FnMut(usize, usize, &std::path::Path)) -> Result<Option<files::HistoryRecord>, String> + Send + 'static,
) {
    use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
    use std::time::Duration;
    state.file_operation_busy = true;
    state.file_operation_progress = None;
    cx.notify();
    let executor = cx.background_executor().clone();
    cx.spawn_in(window, async move |state, cx| {
        let latest = Arc::new(Mutex::new(None::<crate::windows::FileOperationProgress>));
        let done = Arc::new(AtomicBool::new(false));
        let worker_latest = latest.clone();
        let worker_done = done.clone();
        let worker = executor.spawn(async move {
            let mut report = |completed, total, current: &std::path::Path| {
                *worker_latest.lock().unwrap() = Some(crate::windows::FileOperationProgress {
                    completed, total, current: current.to_path_buf(),
                });
            };
            let result = job(&mut report);
            worker_done.store(true, Ordering::Release);
            result
        });
        while !done.load(Ordering::Acquire) {
            let progress = latest.lock().unwrap().take();
            if let Some(progress) = progress {
                let _ = state.update(cx, |state, cx| {
                    state.file_operation_progress = Some(progress);
                    cx.notify();
                });
            }
            executor.timer(Duration::from_millis(80)).await;
        }
        let result = worker.await;
        let _ = state.update_in(cx, |state, window, cx| {
            state.file_operation_busy = false;
            state.file_operation_progress = None;
            match result {
                Ok(record) => {
                    state.file_dialog = None;
                    if let Some(record) = record {
                        state.undo_history.push(record);
                    }
                    state.redo_history.clear();
                    state.reconcile_entries_after_operation();
                    cx.notify();
                }
                Err(error) => {
                    let path = state.current_dir.clone();
                    state.load_directory(path, window, cx);
                    if let Some(dialog) = state.file_dialog.as_mut() {
                        dialog.error = Some(error);
                        cx.notify();
                    } else {
                        message(state, "File operation failed", error, window, cx);
                    }
                }
            }
        });
    }).detach();
}

pub fn move_paths_to(
    paths: Vec<PathBuf>,
    destination: PathBuf,
    state: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    if paths.is_empty() || paths.iter().any(|path| !path.exists()) { return; }
    if files::has_conflicts(&paths, &destination) {
        dialog::show(state, Kind::Conflicts { sources: paths, directory: destination, cut: true, link: false }, "File conflict", "A destination with the same name already exists. Choose how to handle conflicts.", None, window, cx);
    } else {
        transfer_paths_to(paths, destination, true, false, files::ConflictPolicy::Replace, state, window, cx);
    }
}

pub fn copy_paths_to(
    paths: Vec<PathBuf>,
    destination: PathBuf,
    state: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    if paths.is_empty() || paths.iter().any(|path| !path.exists()) { return; }
    if files::has_conflicts(&paths, &destination) {
        dialog::show(state, Kind::Conflicts { sources: paths, directory: destination, cut: false, link: false }, "File conflict", "A destination with the same name already exists. Choose how to handle conflicts.", None, window, cx);
    } else {
        transfer_paths_to(paths, destination, false, false, files::ConflictPolicy::Replace, state, window, cx);
    }
}

pub fn resolve_conflict_choice(
    state: &mut MainWindow,
    sources: Vec<PathBuf>,
    destination: PathBuf,
    cut: bool,
    link: bool,
    policy: files::ConflictPolicy,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    state.file_dialog = None;
    transfer_paths_to(sources, destination, cut, link, policy, state, window, cx);
}

pub fn resolve_link_conflict_choice(
    state: &mut MainWindow,
    source: PathBuf,
    mut destination: PathBuf,
    policy: files::ConflictPolicy,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    state.file_dialog = None;
    if matches!(policy, files::ConflictPolicy::Skip) {
        state.focus.focus(window, cx);
        cx.notify();
        return;
    }
    if matches!(policy, files::ConflictPolicy::KeepBoth) || source == destination {
        if let Some(name) = destination.file_name() {
            destination = files::keep_both_name(destination.parent().unwrap_or_else(|| std::path::Path::new(".")), name);
        }
    }
    perform_progress_recorded(state, window, cx, move |progress| {
        let replaced = if matches!(policy, files::ConflictPolicy::Replace) && source != destination {
            files::trash_with_progress(std::slice::from_ref(&destination), progress).map_err(|error| error.to_string())?;
            vec![destination.clone()]
        } else { Vec::new() };
        progress(0, 1, &destination);
        if let Err(error) = files::create_link(&source, &destination) {
            let _ = files::restore_originals(&replaced);
            return Err(error.to_string());
        }
        progress(1, 1, &destination);
        let mut undo = vec![files::HistoryAction::Remove(vec![destination.clone()])];
        if !replaced.is_empty() { undo.push(files::HistoryAction::Restore(replaced.clone())); }
        let mut redo = Vec::new();
        if !replaced.is_empty() { redo.push(files::HistoryAction::Trash(replaced)); }
        redo.push(files::HistoryAction::Link(vec![(source, destination)]));
        Ok(Some(files::HistoryRecord { undo: files::HistoryAction::Sequence(undo), redo: files::HistoryAction::Sequence(redo) }))
    });
}

pub fn resolve_rename_conflict_choice(
    state: &mut MainWindow,
    source: PathBuf,
    mut destination: PathBuf,
    policy: files::ConflictPolicy,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    state.file_dialog = None;
    if matches!(policy, files::ConflictPolicy::Skip) {
        state.focus.focus(window, cx);
        cx.notify();
        return;
    }
    if matches!(policy, files::ConflictPolicy::KeepBoth) {
        if let Some(name) = destination.file_name() {
            destination = files::keep_both_name(destination.parent().unwrap_or_else(|| std::path::Path::new(".")), name);
        }
    }
    perform_progress_recorded(state, window, cx, move |progress| {
        let replaced = if matches!(policy, files::ConflictPolicy::Replace) {
            files::trash_with_progress(std::slice::from_ref(&destination), progress).map_err(|error| error.to_string())?;
            vec![destination.clone()]
        } else { Vec::new() };
        progress(0, 1, &source);
        if let Err(error) = files::rename(&source, &destination) {
            let _ = files::restore_originals(&replaced);
            return Err(error.to_string());
        }
        progress(1, 1, &destination);
        let mut undo = vec![files::HistoryAction::Move(vec![(destination.clone(), source.clone())])];
        if !replaced.is_empty() { undo.push(files::HistoryAction::Restore(replaced.clone())); }
        let mut redo = Vec::new();
        if !replaced.is_empty() { redo.push(files::HistoryAction::Trash(replaced)); }
        redo.push(files::HistoryAction::Move(vec![(source, destination)]));
        Ok(Some(files::HistoryRecord { undo: files::HistoryAction::Sequence(undo), redo: files::HistoryAction::Sequence(redo) }))
    });
}

fn transfer_paths_to(
    sources: Vec<PathBuf>, destination: PathBuf, cut: bool, link: bool,
    policy: files::ConflictPolicy, state: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>,
) {
    perform_progress_recorded(state, window, cx, move |progress| {
        if cut {
            let (pairs, replaced) = files::move_resolving(&sources, &destination, policy, progress).map_err(|e| e.to_string())?;
            if pairs.is_empty() && replaced.is_empty() { return Ok(None); }
            let mut undo = vec![files::HistoryAction::Move(pairs.iter().map(|(from,to)| (to.clone(),from.clone())).collect())];
            if !replaced.is_empty() { undo.push(files::HistoryAction::Restore(replaced.clone())); }
            let mut redo = Vec::new();
            if !replaced.is_empty() { redo.push(files::HistoryAction::Trash(replaced)); }
            redo.push(files::HistoryAction::Move(pairs));
            Ok(Some(files::HistoryRecord { undo: files::HistoryAction::Sequence(undo), redo: files::HistoryAction::Sequence(redo) }))
        } else {
            let receipt = files::paste_resolving(&sources, &destination, link, policy, progress).map_err(|e| e.to_string())?;
            if receipt.created.is_empty() && receipt.replaced.is_empty() { return Ok(None); }
            let mut undo = vec![files::HistoryAction::Remove(receipt.created.clone())];
            if !receipt.replaced.is_empty() { undo.push(files::HistoryAction::Restore(receipt.replaced.clone())); }
            let mut redo = Vec::new();
            if !receipt.replaced.is_empty() { redo.push(files::HistoryAction::Trash(receipt.replaced)); }
            redo.push(if link { files::HistoryAction::Link(receipt.pairs) } else { files::HistoryAction::Copy(receipt.pairs) });
            Ok(Some(files::HistoryRecord { undo: files::HistoryAction::Sequence(undo), redo: files::HistoryAction::Sequence(redo) }))
        }
    });
}

fn perform_and_navigate(
    state: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
    destination: PathBuf,
    archive: PathBuf,
    job: impl FnOnce() -> Result<(), String> + Send + 'static,
) {
    state.file_operation_busy = true;
    state.file_operation_progress = None;
    cx.notify();
    let executor = cx.background_executor().clone();
    cx.spawn_in(window, async move |state, cx| {
        let result = executor.spawn(async move { job() }).await;
        let _ = state.update_in(cx, |state, window, cx| {
            state.file_operation_busy = false;
            state.file_operation_progress = None;
            match result {
                Ok(()) => {
                    state.undo_history.push(files::HistoryRecord {
                        undo: files::HistoryAction::Remove(vec![destination.clone()]),
                        redo: files::HistoryAction::Extract { archive, destination: destination.clone() },
                    });
                    state.redo_history.clear();
                    state.navigate_to(destination, window, cx)
                }
                Err(error) => message(state, "Could not open ZIP archive", error, window, cx),
            }
        });
    }).detach();
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
