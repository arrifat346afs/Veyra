//! File properties dialog with common filesystem details and type-specific metadata.

pub mod image_metadata;
pub mod system;

use std::{fs, path::Path, time::{SystemTime, UNIX_EPOCH}};
use gpui_kit::*;
use gpui_kit::component::{ActiveTheme as _, button::{Button, ButtonVariants}};
use gpui_kit::prelude::*;
use crate::windows::MainWindow;

pub fn open(path: std::path::PathBuf, state: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    state.properties_dialog = Some(path.clone());
    state.properties_image_details = false;
    state.properties_metadata = None;
    state.properties_metadata_loading = true;
    state.properties_metadata_error = None;
    state.properties_folder_size = None;
    state.properties_filesystem = None;
    state.properties_error = None;
    let executor = cx.background_executor().clone();
    cx.spawn_in(window, async move |state, cx| {
        let metadata_path = path.clone();
        let result = executor.spawn(async move {
            let embedded = image_metadata::read(&metadata_path);
            let size = fs::symlink_metadata(&metadata_path).ok().filter(|meta| meta.is_dir()).map(|_| system::folder_size(&metadata_path));
            let filesystem = system::filesystem_space(&metadata_path);
            (embedded, size, filesystem)
        }).await;
        let _ = state.update(cx, |state, cx| {
            if state.properties_dialog.as_ref() == Some(&path) {
                state.properties_metadata_loading = false;
                let (embedded, size, filesystem) = result;
                state.properties_filesystem = filesystem;
                if let Some(size) = size {
                    match size { Ok(size) => state.properties_folder_size = Some(size), Err(error) => state.properties_error = Some(error) }
                }
                match embedded {
                    Ok(sections) => state.properties_metadata = Some(sections),
                    Err(error) => state.properties_metadata_error = Some(error),
                }
                cx.notify();
            }
        });
    }).detach();
    cx.notify();
}

pub fn view(path: &Path, state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let palette = cx.theme().clone();
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let metadata = fs::symlink_metadata(path).ok();
    let is_directory = metadata.as_ref().is_some_and(|meta| meta.is_dir());
    let is_symlink = metadata.as_ref().is_some_and(|meta| meta.file_type().is_symlink());
    let mime = mime_guess::from_path(path).first().map(|m| m.essence_str().to_owned());
    let kind = if metadata.as_ref().is_some_and(|meta| meta.file_type().is_symlink()) { "Symbolic link".to_owned() }
        else if is_directory { "Folder".into() }
        else { mime.as_ref().map(|value| friendly_type(path, value)).unwrap_or_else(|| "File".into()) };
    let preview_icon = Path::new(&crate::layouts::content::content::papirus_icon(path, is_directory)).to_path_buf();
    let preview = crate::thumbnails::preview(path, preview_icon, true, 88., state.thumbnail_generation);
    let image_dimensions = (!is_directory).then(|| image::image_dimensions(path).ok()).flatten();
    let (owner, group) = metadata.as_ref().map(system::owner_group).unwrap_or_else(|| ("Unavailable".into(), "Unavailable".into()));
    let mode = {
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; (!is_symlink).then(|| metadata.as_ref().map(|meta| meta.permissions().mode() & 0o7777).unwrap_or(0)).unwrap_or(0) }
        #[cfg(not(unix))] { 0 }
    };
    let mut metadata_cards: Vec<AnyElement> = Vec::new();
    if let Some((width, height)) = image_dimensions {
        metadata_cards.push(metadata_section("Image", vec![("Dimensions", format!("{width} × {height} pixels")), ("Format", mime.clone().unwrap_or_else(|| "Unknown".into()))], cx).into_any_element());
    }
    for section in state.properties_metadata.as_ref().into_iter().flatten() {
        metadata_cards.push(metadata_section(section.title, section.fields.clone(), cx).into_any_element());
    }

    if state.properties_image_details {
        return div().absolute().inset_0().size_full().flex().items_center().justify_center().bg(rgba(0x000000bb)).occlude()
            .on_mouse_down(gpui_kit::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().id("image-properties-dialog").w(px(480.)).max_w(relative(0.94)).p(px(20.)).rounded(cx.theme().radius)
                .border_1().border_color(palette.border).bg(palette.popover).text_color(palette.popover_foreground).shadow_lg().flex().flex_col().gap(px(12.))
                .child(div().w_full().h(px(40.)).flex().items_center().justify_between()
                    .child(Button::new("image-properties-back").label("‹  Back").ghost().on_click(cx.listener(|state, _, _, cx| { state.properties_image_details = false; cx.notify(); })))
                    .child(div().flex_1().text_size(px(16.)).font_weight(gpui_kit::FontWeight::BOLD).text_center().child(if image_dimensions.is_some() {"Image Properties"} else {"File Metadata"}))
                    .child(Button::new("close-image-properties").label("×").ghost().on_click(cx.listener(|state, _, window, cx| {
                        state.properties_dialog = None; state.properties_image_details = false; state.properties_metadata = None;
                        state.properties_metadata_error = None; state.properties_folder_size = None; state.properties_filesystem = None; state.properties_error = None; state.properties_metadata_loading = false; state.focus.focus(window, cx); cx.notify();
                    }))))
                .child(div().id("properties-metadata-scroll").max_h(relative(0.68)).overflow_y_scroll().flex().flex_col().gap(px(10.))
                    .children(metadata_cards)
                    .children(state.properties_metadata_loading.then(|| div().p(px(14.)).text_color(palette.muted_foreground).child("Reading embedded metadata…")))
                    .children(state.properties_metadata.as_ref().filter(|fields| fields.is_empty()).map(|_| div().p(px(14.)).text_color(palette.muted_foreground).child("No additional embedded metadata was found.")))
                    .children(state.properties_metadata_error.as_ref().map(|error| div().p(px(14.)).text_color(palette.muted_foreground).child(error.clone()))))
                .child(div().text_size(px(12.)).text_color(palette.muted_foreground).child(name)))
            .into_any_element();
    }

    let mut details = div().flex().flex_col().gap(px(1.))
        .child(info_row("Location", path.display().to_string(), cx))
        .when(is_directory, |rows| rows
            .child(info_row("Contains", format!("{} items", fs::read_dir(path).map(|items| items.count()).unwrap_or(0)), cx))
            .child(info_row("Folder size", state.properties_folder_size.map(system::format_bytes).unwrap_or_else(|| if state.properties_metadata_loading {"Calculating…".into()} else {"Unavailable".into()}), cx)))
        .when_some(metadata.as_ref(), |rows, meta| rows
            .children(meta.accessed().ok().map(|time| info_row("Accessed", format_time(time), cx)))
            .child(info_row("Modified", meta.modified().map(format_time).unwrap_or_else(|_| "Unavailable".into()), cx))
            .child(info_row("Created", meta.created().map(format_time).unwrap_or_else(|_| "Unavailable".into()), cx))
            .child(info_row("Owner", owner, cx))
            .child(info_row("Group", group, cx))
            .child(info_row("Permissions", if is_symlink {"Not applicable".into()} else {format!("{:04o}", mode)}, cx))
            .when(!is_symlink, |rows| rows.child(permission_controls(path, mode, cx)))
            .when(meta.is_file(), |rows| rows.child(info_row("Executable", if mode & 0o111 != 0 {"Yes"} else {"No"}.into(), cx))));
    let filesystem = state.properties_filesystem.as_ref().map(|(kind, _)| kind.clone()).unwrap_or_else(|| if state.properties_metadata_loading {"Checking…".into()} else {"Unavailable".into()});
    let space = state.properties_filesystem.as_ref().map(|(_, space)| space.clone()).unwrap_or_else(|| if state.properties_metadata_loading {"Checking…".into()} else {"Unavailable".into()});
    details = details.child(info_row("Filesystem", filesystem, cx)).child(info_row("Space", space, cx));
    if image_dimensions.is_some() || state.properties_metadata.as_ref().is_some_and(|sections| !sections.is_empty()) || state.properties_metadata_error.is_some() {
        details = details.child(div().w_full().p(px(6.)).rounded(cx.theme().radius).bg(palette.muted.opacity(0.35))
            .child(Button::new("open-image-properties").label(if image_dimensions.is_some() {"Image Properties  →"} else {"File Metadata  →"}).ghost().on_click(cx.listener(|state, _, _, cx| {
                state.properties_image_details = true; cx.notify();
            }))));
    }
    if state.properties_metadata_loading {
        details = details.child(div().text_size(px(12.)).text_color(palette.muted_foreground).child("Reading embedded metadata…"));
    }
    if let Some(error) = &state.properties_error {
        details = details.child(div().text_size(px(12.)).text_color(palette.danger).child(error.clone()));
    }
    if metadata.as_ref().is_some_and(|meta| meta.is_file()) {
        details = details.child(Button::new("set-default-application").label("Set Default Application…").ghost().on_click(cx.listener({ let path = path.to_path_buf(); move |state, _, window, cx| {
            crate::context_menu::application_chooser::open_default(state, path.clone(), window, cx);
        }})));
    }

    div().absolute().inset_0().size_full().flex().items_center().justify_center().bg(rgba(0x000000bb)).occlude()
        .on_mouse_down(gpui_kit::MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(div().id("properties-dialog").w(px(480.)).max_w(relative(0.94)).max_h(relative(0.88)).overflow_y_scroll()
            .p(px(18.)).rounded(cx.theme().radius).border_1().border_color(palette.border).bg(palette.popover).text_color(palette.popover_foreground).shadow_lg()
            .flex().flex_col().gap(px(12.))
            .child(div().w_full().flex().justify_end().child(Button::new("close-properties").label("×").ghost().on_click(cx.listener(|state, _, window, cx| {
                state.properties_dialog = None; state.properties_image_details = false; state.properties_metadata = None;
                state.properties_metadata_error = None; state.properties_folder_size = None; state.properties_filesystem = None; state.properties_error = None; state.properties_metadata_loading = false; state.focus.focus(window, cx); cx.notify();
            }))))
            .child(div().w_full().flex().flex_col().items_center().gap(px(6.)).child(preview)
                .child(div().max_w(px(440.)).whitespace_normal().text_size(px(19.)).font_weight(gpui_kit::FontWeight::BOLD).text_center().child(name))
                .child(div().text_size(px(13.)).text_color(palette.muted_foreground).child(kind))
                .when_some(metadata.as_ref().filter(|meta| meta.is_file()), |row, meta| row.child(div().text_size(px(12.)).text_color(palette.muted_foreground).child(format_size(meta.len())))))
            .child(details)
            .child(div().flex().justify_end().child(Button::new("properties-done").label("Done").primary().on_click(cx.listener(|state, _, window, cx| {
                state.properties_dialog = None; state.properties_image_details = false; state.properties_metadata = None;
                state.properties_metadata_error = None; state.properties_folder_size = None; state.properties_filesystem = None; state.properties_error = None; state.properties_metadata_loading = false; state.focus.focus(window, cx); cx.notify();
            }))))).into_any_element()
}

fn metadata_section(title: &'static str, fields: Vec<(&'static str, String)>, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let palette = cx.theme().clone();
    div().w_full().p(px(14.)).rounded(cx.theme().radius).bg(palette.muted.opacity(0.38))
        .flex().flex_col().gap(px(1.)).child(div().pb(px(6.)).text_size(px(12.)).text_color(palette.muted_foreground).child(title))
        .children(fields.into_iter().map(|(label, value)| {
            div().w_full().flex().flex_col().gap(px(4.)).py(px(7.))
                .child(div().text_size(px(12.)).text_color(palette.muted_foreground).child(label))
                .child(div().text_size(px(14.)).whitespace_normal().child(value))
        }))
}

fn info_row(label: &'static str, value: String, cx: &mut Context<MainWindow>) -> impl IntoElement {
    div().w_full().flex().items_center().justify_between().gap(px(16.)).px(px(12.)).py(px(8.))
        .text_size(px(12.)).child(div().text_color(cx.theme().muted_foreground).child(label))
        .child(div().max_w(px(300.)).text_ellipsis().text_right().child(value))
}

fn permission_controls(path: &Path, mode: u32, cx: &mut Context<MainWindow>) -> impl IntoElement {
    #[cfg(unix)]
    {
        use gpui_kit::component::button::{Button, ButtonVariants};
        let path = path.to_path_buf();
        let mut rows = div().w_full().px(px(12.)).py(px(6.)).flex().flex_col().gap(px(4.))
            .child(div().flex().items_center().gap(px(4.)).child(div().w(px(56.)))
                .child(div().w(px(62.)).text_size(px(10.)).text_color(cx.theme().muted_foreground).child("Read"))
                .child(div().w(px(62.)).text_size(px(10.)).text_color(cx.theme().muted_foreground).child("Write"))
                .child(div().w(px(62.)).text_size(px(10.)).text_color(cx.theme().muted_foreground).child("Execute")));
        for (label, bits) in [("Owner", [0o400, 0o200, 0o100]), ("Group", [0o040, 0o020, 0o010]), ("Others", [0o004, 0o002, 0o001])] {
            let mut row = div().flex().items_center().gap(px(4.)).child(div().w(px(56.)).text_size(px(11.)).child(label));
            for (name, bit) in ["Read", "Write", "Execute"].into_iter().zip(bits) {
                let file = path.clone();
                row = row.child(Button::new(format!("permission-{label}-{name}")).w(px(62.)).label(if mode & bit != 0 {"✓"} else {"—"}).ghost().on_click(cx.listener(move |state, _, _, cx| {
                    match system::toggle_permission(&file, bit) {
                        Ok(()) => state.properties_error = None,
                        Err(error) => state.properties_error = Some(error),
                    }
                    cx.notify();
                })));
            }
            rows = rows.child(row);
        }
        rows
    }
    #[cfg(not(unix))]
    { let _ = (path, mode, cx); div() }
}

fn friendly_type(path: &Path, mime: &str) -> String {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_uppercase();
    let category = mime.split('/').next().unwrap_or("file");
    match (category, extension.as_str()) {
        ("image", _) if !extension.is_empty() => format!("{extension} image"),
        ("image", _) => "Image".into(),
        ("audio", _) if !extension.is_empty() => format!("{extension} audio"),
        ("audio", _) => "Audio".into(),
        ("video", _) if !extension.is_empty() => format!("{extension} video"),
        ("video", _) => "Video".into(),
        (_, "PDF") => "PDF document".into(),
        (_, "DOC" | "DOCX" | "ODT") => "Text document".into(),
        (_, "XLS" | "XLSX" | "ODS" | "CSV") => "Spreadsheet".into(),
        (_, "PPT" | "PPTX" | "ODP") => "Presentation".into(),
        _ if extension.is_empty() => "File".into(),
        _ => format!("{extension} file"),
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
    let mut amount = bytes as f64; let mut unit = 0;
    while amount >= 1000. && unit < UNITS.len() - 1 { amount /= 1000.; unit += 1; }
    if unit == 0 { format!("{bytes} B") } else { format!("{amount:.1} {}", UNITS[unit]) }
}

fn format_time(time: SystemTime) -> String {
    let Ok(seconds) = time.duration_since(UNIX_EPOCH) else { return "Unknown".into(); };
    let timestamp = seconds.as_secs() as libc::time_t;
    let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
    let mut output = [0i8; 80];
    // libc localtime_r writes into our initialized destination and strftime respects the buffer bound.
    unsafe {
        if libc::localtime_r(&timestamp, local.as_mut_ptr()).is_null() { return "Unknown".into(); }
        if libc::strftime(output.as_mut_ptr(), output.len(), b"%m/%d/%Y, %I:%M:%S %p\0".as_ptr().cast(), local.as_ptr()) == 0 { return "Unknown".into(); }
        std::ffi::CStr::from_ptr(output.as_ptr()).to_string_lossy().into_owned()
    }
}
