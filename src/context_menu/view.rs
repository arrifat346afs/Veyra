//! Menu ordering and labels; handlers are delegated to actions.rs.
use super::actions::{self, Command};
use crate::windows::MainWindow;
use gpui_kit::component::{
    Icon,
    menu::{ContextMenuExt, PopupMenuItem},
};
use gpui_kit::*;
use gpui_kit_assets::IconName;

// Native popup rows default to 26px. Custom labels and matching icon margins
// give command rows and submenu triggers the same comfortable 36px height.
fn label(label: &'static str) -> PopupMenuItem {
    PopupMenuItem::element(move |_, _| {
        div().id(label).h(px(36.)).pr(px(8.)).flex().items_center().aria_label(label).child(label)
    })
}

fn icon(name: IconName) -> Icon {
    Icon::new(name).mx(px(4.)).my(px(12.))
}

fn item(
    label: &'static str,
    icon_name: IconName,
    command: Command,
    owner: WeakEntity<MainWindow>,
) -> PopupMenuItem {
    self::label(label)
        .icon(icon(icon_name))
        .on_click(move |_, window, cx| {
            let _ = owner.update(cx, |state, cx| actions::run(command, state, window, cx));
        })
}

pub fn attach(body: Stateful<Div>, cx: &mut Context<MainWindow>) -> AnyElement {
    attach_to(body, None, cx)
}

pub fn attach_entry(
    body: Stateful<Div>,
    path: std::path::PathBuf,
    cx: &mut Context<MainWindow>,
) -> AnyElement {
    attach_to(body, Some(path), cx)
}
fn attach_to(
    body: Stateful<Div>,
    path: Option<std::path::PathBuf>,
    cx: &mut Context<MainWindow>,
) -> AnyElement {
    let owner = cx.entity().downgrade();
    body.context_menu(move |menu, window, cx| {
        let _ = owner.update(cx, |state, cx| {
            state.focus.focus(window, cx);
            if let Some(path) = &path {
                if !state.selection.selected.contains(path) {
                    state.selection.selected.clear();
                    state.selection.selected.insert(path.clone());
                }
            } else {
                state.selection.selected.clear();
            }
            cx.notify();
        });
        let Some(state) = owner.upgrade() else { return menu; };
        let state = state.read(cx);
        let mut paths: Vec<_> = state.selection.selected.iter().cloned().collect();
        paths.sort();
        let busy = state.file_operation_busy;
        let hidden = state.show_hidden;
        let in_trash = super::files::is_trash_location(&state.current_dir);
        if busy {
            return menu.min_w(px(252.)).item(label("Working…").disabled(true));
        }
        let paste_enabled = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .is_some_and(|text| !super::files::clipboard_paths(&text).is_empty());
        let mut menu = menu.min_w(px(252.));
        if !paths.is_empty() {
            if in_trash {
                menu = menu
                    .item(item("Restore", IconName::RefreshCw, Command::Restore, owner.clone()))
                    .item(item("Delete permanently…", IconName::Trash, Command::Trash, owner.clone()));
            } else {
                menu = selection_menu(menu, &paths, state.current_dir.clone(), owner.clone(), window, cx);
            }
            return menu.separator().item(item("Properties", IconName::Info, Command::Properties, owner.clone()));
        }

        if in_trash {
            return menu.item(item("Empty Trash…", IconName::Trash, Command::EmptyTrash, owner.clone()))
                .item(item("Reload", IconName::RefreshCw, Command::Reload, owner.clone()));
        }
        let document_owner = owner.clone();
        let directory = state.current_dir.clone();
        let menu = menu.item(item("New folder", IconName::FolderPlus, Command::NewFolder, owner.clone()))
        .submenu_with_icon(
            Some(icon(IconName::FilePlus)),
            "New document",
            window,
            cx,
            move |menu, _, _| {
                menu.min_w(px(252.)).item(item(
                    "Empty text document",
                    IconName::FilePlus,
                    Command::NewDocument,
                    document_owner.clone(),
                ))
            },
        )
        .item(
            item("Paste", IconName::Clipboard, Command::Paste, owner.clone())
                .disabled(!paste_enabled),
        )
        .item(
            item(
                "Paste as link",
                IconName::Link,
                Command::PasteLink,
                owner.clone(),
            )
            .disabled(!paste_enabled),
        )
        .separator()
        .item(item(
            "Select all",
            IconName::SquareDashed,
            Command::SelectAll,
            owner.clone(),
        ))
        .item(
            item(
                "Show hidden files",
                IconName::Eye,
                Command::Hidden,
                owner.clone(),
            )
            .checked(hidden),
        )
        .item(item(
            "Reload",
            IconName::RefreshCw,
            Command::Reload,
            owner.clone(),
        ))
        .item(item(
            "Open in terminal",
            IconName::Terminal,
            Command::Terminal,
            owner.clone(),
        ))
        .separator()
        .item(item("Properties", IconName::Info, Command::Properties, owner.clone()));
        super::scripts::append(menu, directory, Vec::new(), owner.clone(), window, cx)
    })
    .into_any_element()
}

fn selection_menu(
    mut menu: gpui_kit::component::menu::PopupMenu,
    paths: &[std::path::PathBuf],
    directory: std::path::PathBuf,
    owner: WeakEntity<MainWindow>,
    window: &mut Window,
    cx: &mut Context<gpui_kit::component::menu::PopupMenu>,
) -> gpui_kit::component::menu::PopupMenu {
    let single = paths.len() == 1;
    let one_directory = single && paths[0].is_dir();
    let one_archive = single && crate::services::archives::is_archive(&paths[0]);
    let files_only = paths.iter().all(|path| path.is_file());

    if files_only {
        menu = menu.item(item("Open with…", IconName::AppWindow, Command::ChooseApplication, owner.clone()));
    }
    if single || files_only {
        menu = menu.item(item("Open", IconName::Folder, Command::Open, owner.clone()));
    }
    if single && paths[0].is_file() {
        menu = menu.item(item("Quick preview", IconName::Eye, Command::Preview, owner.clone()));
    }
    if one_archive {
        menu = menu.item(item("Extract to…", IconName::Archive, Command::ExtractArchive, owner.clone()));
    }
    let zip_owner = owner.clone();
    let tar_owner = owner.clone();
    menu = menu.submenu_with_icon(Some(icon(IconName::Archive)), "Compress…", window, cx, move |menu, _, _| {
        menu.min_w(px(252.))
            .item(item("ZIP archive…", IconName::Archive, Command::CompressZip, zip_owner.clone()))
            .item(item("TAR archive…", IconName::Archive, Command::CompressTar, tar_owner.clone()))
    });
    if one_directory {
        menu = menu.item(item("Add to bookmarks", IconName::Bookmark, Command::Bookmark, owner.clone()));
        menu = menu.item(item("Set custom folder icon…", IconName::Folder, Command::SetFolderIcon, owner.clone()));
        if crate::services::folder_icons::get(&paths[0]).is_some() {
            menu = menu.item(item("Reset custom folder icon", IconName::Folder, Command::ResetFolderIcon, owner.clone()));
        }
    }
    menu = menu.separator()
        .item(item("Toggle Starred", IconName::Star, Command::ToggleStar, owner.clone()))
        .item(item("Cut", IconName::Scissors, Command::Cut, owner.clone()))
        .item(item("Copy", IconName::Copy, Command::Copy, owner.clone()))
        .item(item("Copy Path", IconName::Copy, Command::CopyPath, owner.clone()));
    if single {
        menu = menu.item(item("Create symbolic link…", IconName::Link, Command::CreateLink, owner.clone()))
            .item(item("Rename…", IconName::Pencil, Command::Rename, owner.clone()));
    } else {
        menu = menu.item(item("Batch rename…", IconName::Pencil, Command::BatchRename, owner.clone()));
    }
    menu = menu.separator()
        .item(item("Move to trash", IconName::Trash, Command::Trash, owner.clone()))
        .item(item("Delete permanently…", IconName::Trash, Command::Delete, owner.clone()));
    super::scripts::append(menu, directory, paths.to_vec(), owner, window, cx)
}
