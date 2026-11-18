//! Hit testing uses painted tile bounds, so grid, list and scroll offsets agree.
use gpui_kit::*;
use gpui_kit::prelude::*;
use gpui_kit::component::ActiveTheme as _;
use std::path::PathBuf;
use crate::windows::MainWindow;
use super::{Selection,state::rectangle};
pub fn item(tile: Stateful<Div>, path: PathBuf, selection: &Selection, picker: bool, cx: &mut Context<MainWindow>) -> AnyElement {
    let measured_path_for_menu=path.clone();
    let geometry=selection.geometry.clone();
    let measured_path=path.clone();
    let selected=selection.selected.contains(&path);
    let color=cx.theme().primary;
    // A translucent accent keeps hover distinct from selection and lets the
    // pane's background show through, including when window opacity is reduced.
    let hover_color = color.opacity(if selected { 0.28 } else { 0.10 });
    let tile = tile.relative()
        // GPUI allows exactly one hover style per element. Own both selected
        // and unselected hover styling here rather than decorating it twice.
        .when(selected, |tile| tile.bg(color.opacity(0.22)))
        .hover(move |style| style.bg(hover_color))
        .on_mouse_down(MouseButton::Left,cx.listener(move |state,event: &MouseDownEvent,window,cx| {
            state.focus.focus(window,cx);
            state.selection.cursor = Some(path.clone());
            if state.picker.as_ref().is_none_or(|p| p.request.multiple) && (event.modifiers.control || event.modifiers.shift) {
                if !state.selection.selected.insert(path.clone()) {state.selection.selected.remove(&path);}
            } else {
                state.selection.selected.clear();state.selection.selected.insert(path.clone());
            }
            if let Some(picker) = &mut state.picker {
                picker.error = None;
                if picker.request.save && path.is_file() {
                    picker.filename.update(cx, |input,cx| input.set_value(path.file_name().unwrap_or_default().to_string_lossy().to_string(),window,cx));
                }
            }
            cx.stop_propagation();cx.notify();
        }))
        // Anchor to the tile origin; an unanchored absolute child uses its
        // static position after the icon and label, producing incorrect hit bounds.
        .child(canvas(move |bounds,_,_| {geometry.borrow_mut().items.insert(measured_path.clone(),bounds);},|_,_,_,_| {}).absolute().inset_0().size_full())
;
    if picker {tile.into_any_element()} else {crate::context_menu::attach_entry(tile, measured_path_for_menu,cx)}
}
pub fn area(body: AnyElement, selection: &Selection, picker: bool, cx: &mut Context<MainWindow>) -> AnyElement {
    let geometry=selection.geometry.clone();
    let drag=selection.drag;
    let radius=cx.theme().radius;
    let color=cx.theme().primary;
    let area = div().id("selection-area").relative().flex_1().min_h(px(0.)).w_full().overflow_hidden()
        .on_mouse_down(MouseButton::Left,cx.listener(|state,event: &MouseDownEvent,window,cx| {
            state.focus.focus(window,cx);
            if event.modifiers.control || event.modifiers.shift { state.selection.baseline=state.selection.selected.clone(); }
            else {state.selection.baseline.clear();state.selection.selected.clear();}
            if state.picker.as_ref().is_none_or(|p| p.request.multiple) {state.selection.drag=Some((event.position,event.position));}cx.notify();
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
                window.paint_quad(quad(rectangle(start,end),radius,color.opacity(0.12),px(1.),color,BorderStyle::Solid));
            }
        }).absolute().inset_0().size_full())
;
    if picker {area.into_any_element()} else {crate::context_menu::attach(area,cx)}
}
