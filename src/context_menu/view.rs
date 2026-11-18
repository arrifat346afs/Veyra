//! Menu ordering and labels; handlers are delegated to actions.rs.
use super::actions::{self, Command};
use crate::windows::MainWindow;
use gpui_kit::component::{
    Icon,
    menu::{ContextMenuExt, PopupMenuItem},
};
use gpui_kit::*;
use gpui_kit_assets::IconName;

fn item(
    label: &'static str,
    icon: IconName,
    command: Command,
    owner: WeakEntity<MainWindow>,
) -> PopupMenuItem {
    PopupMenuItem::new(label)
        .icon(Icon::new(icon))
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
            return menu.item(PopupMenuItem::new("Working…").disabled(true));
        }
        let paste_enabled = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .is_some_and(|text| !super::files::clipboard_paths(&text).is_empty());
        let hidden = owner
            .upgrade()
            .is_some_and(|state| state.read(cx).show_hidden);
        let document_owner = owner.clone();
        let open_owner = owner.clone();
        let mut menu = menu.min_w(px(215.));
        if path.is_some() {
            menu = menu
                .item(item(
                    "Open",
                    IconName::ExternalLink,
                    Command::Open,
                    owner.clone(),
                ))
                .separator()
                .item(item("Cut", IconName::Scissors, Command::Cut, owner.clone()))
                .item(item("Copy", IconName::Copy, Command::Copy, owner.clone()))
                .item(
                    item("Rename…", IconName::Pencil, Command::Rename, owner.clone())
                        .disabled(count != 1),
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
        menu.item(item(
            "New folder",
            IconName::FolderPlus,
            Command::NewFolder,
            owner.clone(),
        ))
        .submenu_with_icon(
            Some(Icon::new(IconName::FilePlus)),
            "New document",
            window,
            cx,
            move |menu, _, _| {
                menu.item(item(
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
        .submenu_with_icon(
            Some(Icon::new(IconName::AppWindow)),
            "Open with",
            window,
            cx,
            move |menu, _, _| {
                menu.item(item(
                    "Default application",
                    IconName::ExternalLink,
                    Command::Open,
                    open_owner.clone(),
                ))
                .item(item(
                    "Choose application…",
                    IconName::AppWindow,
                    Command::ChooseApplication,
                    open_owner.clone(),
                ))
            },
        )
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
