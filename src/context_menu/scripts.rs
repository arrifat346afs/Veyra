//! Nautilus-compatible user scripts in the file context menu.
use crate::{context_menu::actions, windows::MainWindow};
use gpui_kit::{component::{Icon, menu::{PopupMenu, PopupMenuItem}}, *};
use gpui_kit_assets::IconName;
use gio::prelude::*;
use std::{fs, os::unix::fs::PermissionsExt, path::{Path, PathBuf}, process::Command};

fn scripts() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else { return Vec::new() };
    let Ok(entries) = fs::read_dir(home.join(".local/share/nautilus/scripts")) else { return Vec::new() };
    let mut scripts: Vec<_> = entries.flatten().map(|entry| entry.path()).filter(|path| {
        !path.file_name().is_some_and(|name| name.to_string_lossy().starts_with('.'))
            && path.metadata().is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
    }).collect();
    scripts.sort();
    scripts
}

pub fn append(
    menu: PopupMenu,
    directory: PathBuf,
    selected: Vec<PathBuf>,
    owner: WeakEntity<MainWindow>,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let scripts = scripts();
    if scripts.is_empty() { return menu; }
    menu.submenu_with_icon(Some(Icon::new(IconName::Terminal).mx(px(4.)).my(px(12.))), "Scripts", window, cx, move |mut menu, _, _| {
        for script in &scripts {
            let name = script.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let item_name = name.clone();
            let script = script.clone();
            let selected = selected.clone();
            let directory = directory.clone();
            let owner = owner.clone();
            menu = menu.item(PopupMenuItem::element(move |_, _| {
                div().id(format!("nautilus-script-{item_name}")).h(px(36.)).pr(px(8.)).flex().items_center().child(item_name.clone())
            }).on_click(move |_, window, cx| {
                if let Err(error) = run(&script, &directory, &selected) {
                    let _ = owner.update(cx, |state, cx| actions::message(state, "Could not run script", error, window, cx));
                }
            }));
        }
        menu
    })
}

fn run(script: &Path, directory: &Path, selected: &[PathBuf]) -> Result<(), String> {
    let selected_paths = selected.iter().map(|path| path.to_string_lossy()).collect::<Vec<_>>().join("\n");
    let selected_uris = selected.iter().map(|path| gio::File::for_path(path).uri().to_string()).collect::<Vec<_>>().join("\n");
    let current_uri = gio::File::for_path(directory).uri().to_string();
    Command::new(script)
        .args(selected)
        .current_dir(directory)
        .env("NAUTILUS_SCRIPT_SELECTED_FILE_PATHS", selected_paths)
        .env("NAUTILUS_SCRIPT_SELECTED_URIS", selected_uris)
        .env("NAUTILUS_SCRIPT_CURRENT_URI", current_uri)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("{}: {error}", script.display()))
}
