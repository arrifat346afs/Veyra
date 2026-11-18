//! One navigation surface with a rounded opening over the continuous background.
use std::{cell::Cell, rc::Rc};
use gpui_kit::*;

#[derive(Clone, Default)]
pub struct Frame(Rc<Cell<Option<Bounds<Pixels>>>>);

impl Frame {
    pub fn measure_pane(&self) -> impl IntoElement {
        let pane = self.0.clone();
        canvas(move |bounds, _, _| pane.set(Some(bounds)), |_, _, _, _| {})
            .absolute().inset_0().size_full()
    }

    pub fn background(&self, color: Hsla, radius: Pixels) -> impl IntoElement {
        let pane = self.0.clone();
        canvas(|_, _, _| (), move |bounds, _, window, _| {
            // Every child's prepaint runs before painting begins, so these are
            // the current pane bounds even during resize or sidebar toggling.
            let Some(pane) = pane.get() else { return; };
            let radius = radius.min(pane.size.width / 2.).min(pane.size.height / 2.);
            let mut path = PathBuilder::fill();
            path.move_to(bounds.origin);
            path.line_to(bounds.top_right());
            path.line_to(bounds.bottom_right());
            path.line_to(bounds.bottom_left());
            path.close();

            // Opposite winding makes a hole, rather than painting another
            // background over the pane and changing its effective opacity.
            let corner = |path: &mut PathBuilder, to| {
                if radius.is_zero() { path.line_to(to); }
                else { path.arc_to(point(radius, radius), px(0.), false, false, to); }
            };
            path.move_to(pane.origin + point(radius, px(0.)));
            corner(&mut path, pane.origin + point(px(0.), radius));
            path.line_to(pane.bottom_left() - point(px(0.), radius));
            corner(&mut path, pane.bottom_left() + point(radius, px(0.)));
            path.line_to(pane.bottom_right() - point(radius, px(0.)));
            corner(&mut path, pane.bottom_right() - point(px(0.), radius));
            path.line_to(pane.top_right() + point(px(0.), radius));
            corner(&mut path, pane.top_right() - point(radius, px(0.)));
            path.close();
            if let Ok(path) = path.build() { window.paint_path(path, color); }
        }).absolute().inset_0().size_full()
    }
}
