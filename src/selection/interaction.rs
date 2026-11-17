//! Hit testing uses painted tile bounds, so grid, list and scroll offsets agree.
use gpui_kit::*;
use gpui_kit::prelude::*;
use gpui_kit::component::ActiveTheme as _;
use std::path::PathBuf;
use crate::windows::MainWindow;
use super::{Selection,state::rectangle};
pub fn item(tile: Stateful<Div>, path: PathBuf, selection: &Selection, cx: &mut Context<MainWindow>) -> AnyElement {
    let geometry=selection.geometry.clone();
    let measured_path=path.clone();
    let selected=selection.selected.contains(&path);
    let color=cx.theme().primary;
    let hover_color = if selected { color.opacity(0.28) } else { cx.theme().list_hover };
    tile.relative()
        // GPUI allows exactly one hover style per element. Own both selected
        // and unselected hover styling here rather than decorating it twice.
        .when(selected, |tile| tile.bg(color.opacity(0.22)))
        .hover(move |style| style.bg(hover_color))
        .on_mouse_down(MouseButton::Left,cx.listener(move |state,event: &MouseDownEvent,_,cx| {
            if event.modifiers.control || event.modifiers.shift {
                if !state.selection.selected.insert(path.clone()) {state.selection.selected.remove(&path);}
            } else {
                state.selection.selected.clear();state.selection.selected.insert(path.clone());
            }
            cx.stop_propagation();cx.notify();
        }))
        // Anchor to the tile origin; an unanchored absolute child uses its
        // static position after the icon and label, producing incorrect hit bounds.
        .child(canvas(move |bounds,_,_| {geometry.borrow_mut().items.insert(measured_path.clone(),bounds);},|_,_,_,_| {}).absolute().inset_0().size_full())
        .into_any_element()
}
pub fn area(body: AnyElement, selection: &Selection, cx: &mut Context<MainWindow>) -> AnyElement {
    let geometry=selection.geometry.clone();
    let drag=selection.drag;
    let color=cx.theme().primary;
    div().id("selection-area").relative().flex_1().min_h(px(0.)).w_full().overflow_hidden()
        .on_mouse_down(MouseButton::Left,cx.listener(|state,event: &MouseDownEvent,_,cx| {
            if event.modifiers.control || event.modifiers.shift { state.selection.baseline=state.selection.selected.clone(); }
            else {state.selection.baseline.clear();state.selection.selected.clear();}
            state.selection.drag=Some((event.position,event.position));cx.notify();
        }))
        .on_mouse_move(cx.listener(|state,event: &MouseMoveEvent,_,cx| {
            if state.selection.drag.is_some() {state.selection.update(event.position);cx.notify();}
        }))
        .on_mouse_up(MouseButton::Left,cx.listener(|state,event: &MouseUpEvent,_,cx| {
            if state.selection.drag.is_some() {state.selection.update(event.position);state.selection.drag=None;cx.notify();}
        }))
        .on_mouse_up_out(MouseButton::Left,cx.listener(|state,event: &MouseUpEvent,_,cx| {
            if state.selection.drag.is_some() {state.selection.update(event.position);state.selection.drag=None;cx.notify();}
        }))
        .child(body)
        // The measurement canvas must cover the viewport from its top-left,
        // rather than starting after the full-height content child.
        .child(canvas(move |bounds,_,_| {geometry.borrow_mut().viewport=bounds;},move |_,_,window,_| {
            if let Some((start,end))=drag {
                window.paint_quad(quad(rectangle(start,end),px(2.),color.opacity(0.12),px(1.),color,BorderStyle::Solid));
            }
        }).absolute().inset_0().size_full())
        .into_any_element()
}
