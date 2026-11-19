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
        let count = owner
            .upgrade()
            .map(|state| state.read(cx).selection.selected.len())
            .unwrap_or(0);
        let busy = owner
            .upgrade()
            .is_some_and(|state| state.read(cx).file_operation_busy);
        if busy {
            return menu.min_w(px(252.)).item(label("Working…").disabled(true));
        }
        let paste_enabled = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .is_some_and(|text| !super::files::clipboard_paths(&text).is_empty());
        let hidden = owner
            .upgrade()
            .is_some_and(|state| state.read(cx).show_hidden);
        let document_owner = owner.clone();
        // Empty-space menus apply Open with to the current folder; entry
        // menus apply it to the clicked item or the existing multi-selection.
        let mut menu = menu.min_w(px(252.))
            .item(item(
                "Open with…",
                IconName::AppWindow,
                Command::ChooseApplication,
                owner.clone(),
            ))
            .separator();
        if path.is_some() {
            let zip_selected = path.as_ref().is_some_and(|path| super::files::is_zip(path));
            menu = menu
                .item(item("Quick preview",IconName::Eye,Command::Preview,owner.clone()).disabled(count != 1 || path.as_ref().is_some_and(|p| !p.is_file())))
                .item(item("Extract to folder", IconName::Archive, Command::ExtractZip, owner.clone()).disabled(count != 1 || !zip_selected))
                .item(item("Cut", IconName::Scissors, Command::Cut, owner.clone()))
                .item(item("Copy", IconName::Copy, Command::Copy, owner.clone()))
                .item(
                    item("Rename…", IconName::Pencil, Command::Rename, owner.clone())
                        .disabled(count != 1),
                )
                .item(
                    item("Batch rename…", IconName::Pencil, Command::BatchRename, owner.clone())
                        .disabled(count < 2),
                )
                .item(item(
                    "Move to trash",
                    IconName::Trash,
                    Command::Trash,
                    owner.clone(),
                ))
                .item(item(
                    "Delete permanently…",
                    IconName::Trash,
                    Command::Delete,
                    owner.clone(),
                ))
                .separator();
        }
        if path.as_ref().is_some_and(|path| path.is_dir()) {
            menu = menu.item(item("Add to bookmarks", IconName::Bookmark, Command::Bookmark, owner.clone()))
                .separator();
        }
        menu.item(item(
            "New folder",
            IconName::FolderPlus,
            Command::NewFolder,
            owner.clone(),
        ))
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
        .item(item(
            "Properties",
            IconName::Info,
            Command::Properties,
            owner.clone(),
        ))
    })
    .into_any_element()
}
