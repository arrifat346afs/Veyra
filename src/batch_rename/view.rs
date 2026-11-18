//! Batch-rename dialog view: mode selector, inputs, live preview table, action buttons.
use super::state::{BatchRename, Mode};
use crate::context_menu::actions;
use crate::windows::MainWindow;
use gpui_kit::component::{
    ActiveTheme as _, Disableable,
    button::*,
    input::{Input, InputEvent},
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::path::PathBuf;

/// Open the batch-rename dialog with the currently selected files.
pub fn open(
    state: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let mut paths: Vec<_> = state.selection.selected.iter().cloned().collect();
    paths.sort();
    if paths.len() < 2 {
        return;
    }
    let batch = BatchRename::new(paths, window, cx);

    // Subscribe to both inputs so we can refresh the preview on every change.
    let find = batch.find_input.clone();
    let replace = batch.replace_input.clone();
    cx.subscribe_in(&find, window, |state, _, _: &InputEvent, _, cx| {
        if let Some(batch) = &mut state.batch_rename { batch.refresh(cx); }
        cx.notify();
    }).detach();
    cx.subscribe_in(&replace, window, |state, _, _: &InputEvent, _, cx| {
        if let Some(batch) = &mut state.batch_rename { batch.refresh(cx); }
        cx.notify();
    }).detach();

    batch.find_input.update(cx, |input, cx| input.focus(window, cx));
    state.batch_rename = Some(batch);
    cx.notify();
}

pub fn close(state: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    if state.file_operation_busy { return; }
    state.batch_rename = None;
    state.focus.focus(window, cx);
    cx.notify();
}

fn submit(state: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    if state.file_operation_busy { return; }
    let Some(batch) = &state.batch_rename else { return; };
    if batch.has_errors() { return; }

    // Check that something actually changed.
    if batch.preview.iter().all(|r| r.original == r.renamed) {
        close(state, window, cx);
        return;
    }

    let pairs: Vec<_> = batch.rename_pairs().into_iter().filter(|(s, d)| s != d).map(|(s, d)| {
        let index = batch.paths.iter().position(|path| path == &s).unwrap();
        (s, d, batch.identities[index])
    }).collect();
    actions::perform(state, window, cx, move || {
        use std::os::unix::fs::MetadataExt;
        for (source, _, expected) in &pairs {
            let actual = source.symlink_metadata().map(|m| (m.dev(), m.ino())).map_err(|_| format!("{} changed. Review the selection again.", source.display()))?;
            if actual != *expected { return Err(format!("{} changed. Review the selection again.", source.display())); }
        }
        for (i, (source, dest, _)) in pairs.iter().enumerate() {
            if dest.symlink_metadata().is_ok() && !pairs.iter().any(|(planned, _, _)| planned == dest) {
                return Err(format!("Destination {} is already occupied", dest.display()));
            }
            for (other_source, other_dest, _) in pairs.iter().skip(i + 1) {
                if dest == other_source && other_dest == source { return Err("The rename plan contains a cycle; no files were changed.".into()); }
            }
        }
        let mut pending: Vec<_> = pairs.iter().map(|(s, d, _)| (s.clone(), d.clone())).collect();
        let mut ordered = Vec::new();
        while !pending.is_empty() {
            if let Some(i) = pending.iter().position(|(_, dest)| !pending.iter().any(|(src, _)| src == dest)) {
                ordered.push(pending.remove(i));
            } else { return Err("The rename plan contains a cycle; no files were changed.".into()); }
        }
        let mut succeeded = Vec::new();
        for (source, dest) in &ordered {
            use std::os::unix::fs::MetadataExt;
            let actual = source.symlink_metadata().map(|m| (m.dev(), m.ino())).map_err(|_| format!("{} changed. Review the selection again.", source.display()))?;
            let expected = pairs.iter().find(|(p, _, _)| p == source).unwrap().2;
            if actual != expected { return Err(format!("{} changed. Review the selection again.", source.display())); }
            if let Err(e) = crate::context_menu::files::rename(source, dest) {
                return Err(format!("Rename stopped: {e}. Succeeded: {}. Remaining: {}", succeeded.iter().map(|p: &PathBuf| p.display().to_string()).collect::<Vec<_>>().join(", "), ordered.iter().skip(succeeded.len()).map(|(_, p)| p.display().to_string()).collect::<Vec<_>>().join(", ")));
            }
            succeeded.push(source.clone());
        }
        Ok(())
    });
    state.batch_rename = None;
}

/// Render the batch-rename overlay dialog.
pub fn view(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let batch = state.batch_rename.as_ref().unwrap();
    let palette = cx.theme().clone();
    let busy = state.file_operation_busy;
    let has_errors = batch.has_errors();
    let no_changes = batch.preview.iter().all(|r| r.original == r.renamed);
    let current_mode = batch.mode;

    // --- Mode tabs ---
    let mode_tabs = div().flex().gap(px(4.)).mb(px(8.)).children(
        Mode::ALL.iter().map(|&mode| {
            let active = mode == current_mode;
            Button::new(SharedString::from(format!("batch-mode-{:?}", mode)))
                .label(mode.label())
                .when(active, |b| b.primary())
                .on_click(cx.listener(move |state, _, _, cx| {
                    if let Some(batch) = &mut state.batch_rename {
                        batch.mode = mode;
                        batch.refresh(cx);
                    }
                    cx.notify();
                }))
        })
    );

    // --- Inputs ---
    let inputs = match current_mode {
        Mode::Replace => div().flex().flex_col().gap(px(8.))
            .child(div().text_size(px(13.)).text_color(palette.muted_foreground).child("Existing text"))
            .child(Input::new(&batch.find_input))
            .child(div().text_size(px(13.)).text_color(palette.muted_foreground).child("Replace with"))
            .child(Input::new(&batch.replace_input))
            .into_any_element(),
        Mode::Format => div().flex().flex_col().gap(px(8.))
            .child(div().text_size(px(13.)).text_color(palette.muted_foreground).child("Template"))
            .child(Input::new(&batch.replace_input))
            .child(
                div().text_size(px(12.)).text_color(palette.muted_foreground)
                    .child("[n] = counter   [0n] = 01,02…   [00n] = 001,002…   [name] = original name   [ext] = .extension")
            )
            .into_any_element(),
    };

    // --- Preview table ---
    let header = div().flex().w_full().gap(px(12.)).pb(px(4.))
        .border_b_1().border_color(palette.border)
        .child(div().flex_1().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).child("Original"))
        .child(div().flex_1().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).child("New name"));

    let rows: Vec<_> = batch.preview.iter().enumerate().map(|(i, row)| {
        let error = row.error;
        let changed = row.original != row.renamed;
        div().id(SharedString::from(format!("preview-row-{i}"))).flex().w_full().gap(px(12.)).py(px(3.))
            .when(i % 2 == 0, |d| d.bg(palette.muted.opacity(0.3)))
            .child(
                div().flex_1().text_size(px(13.)).text_ellipsis()
                    .text_color(palette.muted_foreground)
                    .child(row.original.clone())
            )
            .child(
                div().flex_1().text_size(px(13.)).text_ellipsis()
                    .when(error, |d| d.text_color(palette.danger))
                    .when(!error && changed, |d| d.text_color(palette.primary))
                    .when(!error && !changed, |d| d.text_color(palette.muted_foreground))
                    .child(row.renamed.clone())
            )
    }).collect();

    let preview = div().flex_col().gap(px(2.))
        .mt(px(12.))
        .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child(
            format!("Preview ({} files)", batch.paths.len())
        ))
        .child(header)
        .child(
            div().id("batch-preview-scroll")
                .max_h(px(260.))
                .overflow_y_scroll()
                .flex().flex_col()
                .children(rows)
        );

    // --- Error ---
    let error_view = batch.error.clone().map(|error| {
        div().text_size(px(13.)).text_color(palette.danger).child(error)
    });

    // --- Buttons ---
    let buttons = div().flex().justify_end().gap(px(8.)).mt(px(8.))
        .child(
            Button::new("batch-cancel")
                .label("Cancel")
                .disabled(busy)
                .on_click(cx.listener(|s, _, w, c| close(s, w, c)))
        )
        .child(
            Button::new("batch-submit")
                .label(if busy { "Renaming…" } else { "Rename" })
                .primary()
                .disabled(busy || has_errors || no_changes)
                .on_click(cx.listener(|s, _, w, c| submit(s, w, c)))
        );

    // --- Dialog chrome ---
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
                .id("batch-rename-dialog")
                .w(px(600.))
                .max_w(relative(0.94))
                .max_h(relative(0.88))
                .overflow_y_scroll()
                .p(px(22.))
                .rounded(palette.radius)
                .border_1()
                .border_color(palette.border)
                .bg(palette.popover)
                .text_color(palette.popover_foreground)
                .shadow_lg()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(div().text_size(px(20.)).child("Batch Rename"))
                .child(mode_tabs)
                .child(inputs)
                .child(preview)
                .children(error_view)
                .child(buttons)
        )
        .into_any_element()
}
