use gpui_kit::*;
use gpui_kit::component::input::Input;
use crate::windows::MainWindow;
pub fn search(state: &MainWindow) -> AnyElement {
    div().flex_1().min_w(px(100.)).child(Input::new(&state.search)).into_any_element()
}
