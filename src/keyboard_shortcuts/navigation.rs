//! Selection navigation is scoped to the file pane, leaving text inputs alone.
use crate::windows::MainWindow;
use gpui_kit::*;

fn destination(current: Option<usize>, key: &str, len: usize, columns: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let Some(current) = current else {
        return Some(if key == "end" { len - 1 } else { 0 });
    };
    Some(match key {
        "left" => current.saturating_sub(1),
        "right" => (current + 1).min(len - 1),
        "up" => current.saturating_sub(columns),
        "down" => (current + columns).min(len - 1),
        "home" => 0,
        "end" => len - 1,
        _ => return None,
    })
}

pub(super) fn handle(
    state: &mut MainWindow,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) -> bool {
    if state.file_dialog.is_some() || state.path_bar.editing || !state.focus.is_focused(window) {
        return false;
    }
    let key = event.keystroke.key.as_str();
    let modifiers = event.keystroke.modifiers;
    if modifiers
        == (Modifiers {
            alt: true,
            ..Default::default()
        })
        && matches!(key, "left" | "right")
    {
        if !event.is_held {
            if key == "left" {
                state.go_back(window, cx);
            } else {
                state.go_forward(window, cx);
            }
        }
    } else if modifiers != Modifiers::default() {
        return false;
    } else if key == "backspace" {
        if !event.is_held {
            if let Some(parent) = state.current_dir.parent().map(|p| p.to_path_buf()) {
                state.navigate_to(parent, window, cx);
            }
        }
    } else if matches!(key, "left" | "right" | "up" | "down" | "home" | "end") {
        let selection = &mut state.selection;
        let current = selection
            .cursor
            .as_ref()
            .filter(|path| selection.selected.contains(*path))
            .and_then(|path| selection.visible.iter().position(|p| p == path))
            .or_else(|| {
                selection
                    .visible
                    .iter()
                    .position(|p| selection.selected.contains(p))
            });
        // Painted bounds follow the actual wrapping layout, including window
        // resizing, icon sizes, density, and list view.
        let geometry = selection.geometry.borrow();
        let columns = if state.list_view {
            1
        } else {
            selection
                .visible
                .first()
                .and_then(|path| geometry.items.get(path))
                .map(|first| {
                    geometry
                        .items
                        .values()
                        .filter(|bounds| bounds.top() == first.top())
                        .count()
                })
                .unwrap_or(1)
                .max(1)
        };
        if let Some(index) = destination(current, key, selection.visible.len(), columns) {
            let path = selection.visible[index].clone();
            selection.selected.clear();
            selection.selected.insert(path.clone());
            selection.cursor = Some(path.clone());
            if let Some(bounds) = geometry.items.get(&path) {
                let viewport = geometry.viewport;
                let delta = if bounds.top() < viewport.top() {
                    viewport.top() - bounds.top()
                } else if bounds.bottom() > viewport.bottom() {
                    viewport.bottom() - bounds.bottom()
                } else {
                    px(0.)
                };
                let offset = selection.scroll.offset();
                selection
                    .scroll
                    .set_offset(point(offset.x, offset.y + delta));
            }
            if let Some(picker) = &mut state.picker {
                picker.error = None;
                if picker.request.save && path.is_file() {
                    picker.filename.update(cx, |input, cx| {
                        input.set_value(
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            window,
                            cx,
                        )
                    });
                }
            }
        }
        drop(geometry);
        cx.notify();
    } else {
        return false;
    }
    cx.stop_propagation();
    true
}

#[cfg(test)]
mod tests {
    use super::destination;

    #[test]
    fn navigation_starts_selection_and_handles_empty_folders() {
        assert_eq!(destination(None, "right", 0, 3), None);
        for key in ["left", "right", "up", "down", "home"] {
            assert_eq!(destination(None, key, 8, 3), Some(0));
        }
        assert_eq!(destination(None, "end", 8, 3), Some(7));
    }

    #[test]
    fn arrows_follow_rows_and_clamp_at_edges() {
        assert_eq!(destination(Some(0), "right", 8, 3), Some(1));
        assert_eq!(destination(Some(1), "left", 8, 3), Some(0));
        assert_eq!(destination(Some(1), "down", 8, 3), Some(4));
        assert_eq!(destination(Some(4), "up", 8, 3), Some(1));
        assert_eq!(destination(Some(5), "down", 8, 3), Some(7));
        assert_eq!(destination(Some(0), "left", 8, 3), Some(0));
        assert_eq!(destination(Some(7), "right", 8, 3), Some(7));
        assert_eq!(destination(Some(2), "down", 8, 1), Some(3));
        assert_eq!(destination(Some(4), "home", 8, 3), Some(0));
        assert_eq!(destination(Some(4), "end", 8, 3), Some(7));
    }
}
