//! Persistent drag-and-drop ordering for all sidebar places.

use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;
use crate::{services::bookmarks, windows::MainWindow};

#[derive(Clone)]
struct PlaceDrag { key: String, label: String }

impl Render for PlaceDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().px(px(12.)).py(px(8.)).rounded(cx.theme().radius).bg(cx.theme().popover)
            .text_color(cx.theme().popover_foreground).shadow_lg().child(self.label.clone())
    }
}

pub fn rows(mut rows: Vec<(String, String, AnyElement)>, cx: &mut Context<MainWindow>) -> Vec<AnyElement> {
    let saved = bookmarks::get_sidebar_order();
    rows.sort_by_key(|(key, _, _)| saved.iter().position(|item| item == key).unwrap_or(usize::MAX));
    let order: Vec<_> = rows.iter().map(|(key, _, _)| key.clone()).collect();
    let mut result: Vec<_> = rows.into_iter().map(|(key, label, child)| {
        let target = key.clone();
        let order = order.clone();
        div().id(format!("sidebar-sort-{key}")).w_full().border_t_2().border_color(gpui_kit::transparent_black())
            .on_drag(PlaceDrag {key, label}, |drag, _, _, cx| { cx.stop_propagation(); cx.new(|_| drag.clone()) })
            .drag_over::<PlaceDrag>(|style, _, _, cx| style.border_color(cx.theme().primary))
            .on_drop(cx.listener(move |_, drag: &PlaceDrag, _, cx| {
                if let Err(error) = bookmarks::reorder_sidebar(&order, &drag.key, Some(&target)) {
                    eprintln!("Could not reorder sidebar places: {error}");
                }
                cx.notify();
            }))
            .child(child).into_any_element()
    }).collect();
    result.push(div().id("sidebar-sort-end").w_full().h(px(8.)).border_b_2().border_color(gpui_kit::transparent_black())
        .drag_over::<PlaceDrag>(|style, _, _, cx| style.border_color(cx.theme().primary))
        .on_drop(cx.listener(move |_, drag: &PlaceDrag, _, cx| {
            if let Err(error) = bookmarks::reorder_sidebar(&order, &drag.key, None) {
                eprintln!("Could not reorder sidebar places: {error}");
            }
            cx.notify();
        })).into_any_element());
    result
}
