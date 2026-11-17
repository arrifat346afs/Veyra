use gpui_kit::*;
use gpui_kit_assets::IconName;
use crate::windows::MainWindow;
use super::button::button;
pub fn sorting(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    button("sort-direction", if state.reverse_sort {IconName::ArrowDown} else {IconName::ArrowUp}, ["Name", "Size", "Modified", "Type"][state.preferences.sort_by.min(3)], state.preferences.foreground_alpha(), cx)
        .on_click(cx.listener(|s, _, _, cx| {s.reverse_sort = !s.reverse_sort; cx.notify();})).into_any_element()
}
