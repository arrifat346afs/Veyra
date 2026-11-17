use gpui_kit::*;
use std::{cell::RefCell, collections::{HashMap, HashSet}, path::PathBuf, rc::Rc};
#[derive(Default)]
pub struct Geometry {
    pub viewport: Bounds<Pixels>,
    pub items: HashMap<PathBuf, Bounds<Pixels>>,
}
#[derive(Default)]
pub struct Selection {
    pub selected: HashSet<PathBuf>,
    pub geometry: Rc<RefCell<Geometry>>,
    pub drag: Option<(Point<Pixels>, Point<Pixels>)>,
    pub baseline: HashSet<PathBuf>,
}
pub fn rectangle(a: Point<Pixels>, b: Point<Pixels>) -> Bounds<Pixels> {
    Bounds::new(point(a.x.min(b.x),a.y.min(b.y)),size((a.x-b.x).abs(),(a.y-b.y).abs()))
}
impl Selection {
    pub fn update(&mut self, position: Point<Pixels>) {
        if let Some((start,_)) = self.drag {
            let geometry=self.geometry.borrow();
            let bounds=geometry.viewport;
            let end=point(position.x.clamp(bounds.left(),bounds.right()),position.y.clamp(bounds.top(),bounds.bottom()));
            self.drag=Some((start,end));
            let rect=rectangle(start,end);
            self.selected=self.baseline.clone();
            if rect.size.width > px(3.) || rect.size.height > px(3.) {
                for (path,bounds) in &geometry.items {
                    if rect.intersects(bounds) {self.selected.insert(path.clone());}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Selection;
    use gpui_kit::{Bounds, point, px, size};
    use std::path::PathBuf;
    #[test]
    fn reverse_drag_selects_intersections_and_keeps_additive_selection() {
        let mut selection=Selection::default();
        selection.geometry.borrow_mut().viewport=Bounds::new(point(px(0.),px(0.)),size(px(300.),px(300.)));
        selection.geometry.borrow_mut().items.insert("inside".into(),Bounds::new(point(px(40.),px(40.)),size(px(30.),px(30.))));
        selection.geometry.borrow_mut().items.insert("outside".into(),Bounds::new(point(px(220.),px(220.)),size(px(30.),px(30.))));
        selection.baseline.insert("existing".into());
        selection.drag=Some((point(px(100.),px(100.)),point(px(100.),px(100.))));
        selection.update(point(px(20.),px(20.)));
        assert!(selection.selected.contains(&PathBuf::from("inside")));
        assert!(selection.selected.contains(&PathBuf::from("existing")));
        assert!(!selection.selected.contains(&PathBuf::from("outside")));
        selection.update(point(px(90.),px(90.)));
        assert!(!selection.selected.contains(&PathBuf::from("inside")));
    }
    #[test]
    fn offset_viewport_tracks_pointer_in_all_drag_directions() {
        let mut selection = Selection::default();
        selection.geometry.borrow_mut().viewport = Bounds::new(
            point(px(240.), px(120.)), size(px(800.), px(600.)));
        selection.geometry.borrow_mut().items.insert("folder".into(), Bounds::new(
            point(px(420.), px(280.)), size(px(100.), px(100.))));
        for (start, end) in [
            ((300.,200.),(600.,450.)), ((600.,450.),(300.,200.)),
            ((300.,450.),(600.,200.)), ((600.,200.),(300.,450.)),
        ] {
            let start = point(px(start.0), px(start.1));
            let end = point(px(end.0), px(end.1));
            selection.drag = Some((start, start));
            selection.update(end);
            assert_eq!(selection.drag, Some((start, end)));
            assert!(selection.selected.contains(&PathBuf::from("folder")));
        }
        selection.update(point(px(2000.), px(2000.)));
        assert_eq!(selection.drag.unwrap().1, point(px(1040.),px(720.)));
    }

}
