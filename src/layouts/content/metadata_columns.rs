//! Configurable metadata columns for the detailed list view.

use std::{fs, path::Path, time::SystemTime};
use gpui_kit::{div, px, Context, IntoElement, ParentElement, Styled, WeakEntity};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::menu::{ContextMenuExt, PopupMenuItem};
use crate::windows::MainWindow;

const COLUMNS: [(&str, &str); 4] = [("size", "Size"), ("type", "Type"), ("modified", "Modified"), ("permissions", "Permissions")];

pub fn header(owner: WeakEntity<MainWindow>, visible: &[String], cx: &mut Context<MainWindow>) -> impl IntoElement {
    let selected_columns = visible.to_vec();
    let columns: Vec<_> = COLUMNS.iter().filter(|(id, _)| visible.iter().any(|value| value == id)).collect();
    let menu_columns = COLUMNS;
    div().w_full().flex().items_center().gap(px(10.)).px(px(18.)).py(px(7.))
        .child(div().w(px(62.)).flex_shrink_0())
        .child(div().flex_1().child("Name"))
        .children(columns.into_iter().map(|(_, label)| div().w(px(120.)).flex_shrink_0().text_color(cx.theme().muted_foreground).text_size(px(11.)).child(*label)))
        .context_menu(move |mut menu, _, _| {
            for (id, label) in menu_columns {
                let owner = owner.clone(); let id = id.to_owned(); let title = format!("{}{}", if selected_columns.iter().any(|value| value == &id) {"✓  "} else {"    "}, label);
                menu = menu.item(PopupMenuItem::element(move |_, _| div().h(px(36.)).pr(px(8.)).flex().items_center().child(title.clone()))
                    .on_click(move |_, _, cx| { let id = id.clone(); let _ = owner.update(cx, |state, cx| {
                        let selected = &mut state.preferences.metadata_columns;
                        if selected.iter().any(|value| value == &id) { selected.retain(|value| value != &id); }
                        else { selected.push(id); }
                        if let Err(error) = state.preferences.save() { eprintln!("Could not save metadata columns: {error}"); }
                        cx.notify();
                    }); }));
            }
            menu
        })
}

pub fn cells(path: &Path, visible: &[String], cx: &mut Context<MainWindow>) -> impl IntoElement {
    let metadata = fs::symlink_metadata(path).ok();
    div().flex().items_center().gap(px(10.)).children(COLUMNS.iter()
        .filter(|(id, _)| visible.iter().any(|value| value == id)).map(|(id, _)| {
            let value = match (*id, metadata.as_ref()) {
                ("size", Some(meta)) if meta.is_dir() => "—".to_owned(),
                ("size", Some(meta)) => format_size(meta.len()),
                ("type", _) if path.is_dir() => "Folder".to_owned(),
                ("type", _) => mime_guess::from_path(path).first().map(|mime| mime.essence_str().to_owned()).unwrap_or_else(|| "File".into()),
                ("modified", Some(meta)) => relative_time(meta.modified().ok()),
                ("permissions", Some(meta)) => permissions(meta),
                _ => "—".into(),
            };
            div().w(px(120.)).flex_shrink_0().text_ellipsis().text_color(cx.theme().muted_foreground).text_size(px(11.)).child(value)
        }))
}

fn format_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut amount = size as f64; let mut unit = 0;
    while amount >= 1024. && unit < UNITS.len() - 1 { amount /= 1024.; unit += 1; }
    if unit == 0 { format!("{size} B") } else { format!("{amount:.1} {}", UNITS[unit]) }
}

fn relative_time(modified: Option<SystemTime>) -> String {
    let Some(modified) = modified else { return "—".into(); };
    let Ok(age) = modified.elapsed() else { return "Just now".into(); };
    let seconds = age.as_secs();
    if seconds < 60 { "Just now".into() }
    else if seconds < 3600 { format!("{} min ago", seconds / 60) }
    else if seconds < 86400 { format!("{} hr ago", seconds / 3600) }
    else { format!("{} days ago", seconds / 86400) }
}

#[cfg(unix)]
fn permissions(metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::PermissionsExt;
    format!("{:04o}", metadata.permissions().mode() & 0o7777)
}
#[cfg(not(unix))]
fn permissions(metadata: &fs::Metadata) -> String { if metadata.permissions().readonly() { "Read only" } else { "Read/write" }.into() }
