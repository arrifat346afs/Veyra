//! Inline folder expansion for the list view.
use std::{collections::HashMap, path::PathBuf};

use gpui_kit::{div, px, Context, IntoElement, ParentElement, Styled};
use gpui_kit::component::{ActiveTheme as _, Icon};
use gpui_kit::prelude::*;
use gpui_kit_assets::IconName;

use crate::{settings::preferences::Preferences, windows::MainWindow};

pub(super) fn disclosure(path: PathBuf, expanded: bool, cx: &mut Context<MainWindow>) -> impl IntoElement {
    div().id(format!("expand-{}", path.display())).w(px(18.)).h(px(24.))
        .flex_shrink_0().flex().items_center().justify_center().cursor_pointer()
        .on_click(cx.listener(move |state, _, _, cx| {
            cx.stop_propagation();
            state.toggle_expanded_folder(path.clone(), cx);
        }))
        .child(Icon::new(if expanded { IconName::ChevronDown } else { IconName::ChevronRight })
            .size(px(14.)).text_color(cx.theme().muted_foreground))
}

pub(crate) fn flatten_entries(
    roots: &[PathBuf],
    expanded: &HashMap<PathBuf, Vec<PathBuf>>,
    show_hidden: bool,
    query: &str,
    preferences: &Preferences,
) -> Vec<(PathBuf, usize)> {
    fn append(
        path: &PathBuf,
        depth: usize,
        expanded: &HashMap<PathBuf, Vec<PathBuf>>,
        show_hidden: bool,
        query: &str,
        preferences: &Preferences,
        output: &mut Vec<(PathBuf, usize)>,
    ) {
        output.push((path.clone(), depth));
        let Some(children) = expanded.get(path) else { return; };
        let mut children: Vec<_> = children.iter().filter(|child| {
            let name = child.file_name().unwrap_or_default().to_string_lossy();
            (show_hidden || !name.starts_with('.')) && (if preferences.case_sensitive {name.contains(query)} else {name.to_lowercase().contains(query)})
        }).cloned().collect();
        children.sort_by(|a, b| {
            let order = match preferences.sort_by {
                1 => a.metadata().map(|m| m.len()).unwrap_or(0).cmp(&b.metadata().map(|m| m.len()).unwrap_or(0)),
                2 => a.metadata().and_then(|m| m.modified()).ok().cmp(&b.metadata().and_then(|m| m.modified()).ok()),
                3 => a.extension().cmp(&b.extension()),
                _ => a.file_name().cmp(&b.file_name()),
            }.then_with(|| a.file_name().cmp(&b.file_name()));
            let order = if preferences.reverse_sort { order.reverse() } else { order };
            if preferences.folders_first { b.is_dir().cmp(&a.is_dir()).then(order) } else { order }
        });
        for child in children { append(&child, depth + 1, expanded, show_hidden, query, preferences, output); }
    }

    let mut output = Vec::new();
    for root in roots { append(root, 0, expanded, show_hidden, query, preferences, &mut output); }
    output
}
