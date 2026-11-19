use gpui_kit::*;
use gpui_kit::prelude::*;
use crate::windows::MainWindow;
use super::{navigation, search, views, sorting, menu};
use gpui_kit_assets::IconName;

pub fn topbar(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    div().w_full().flex_shrink_0().flex().items_center().gap(px(8.))
        .px(px(12.)).py(px(12.))
        .child(menu::sidebar_toggle(state.preferences.foreground_alpha(), cx))
        .child(navigation::navigation(state, cx))
        .when(crate::context_menu::files::is_trash_location(&state.current_dir), |bar| {
            bar.child(super::button::button("empty-trash", IconName::Trash, "Empty Trash…", state.preferences.foreground_alpha(), cx)
                .on_click(cx.listener(|state, _, window, cx| {
                    crate::context_menu::actions::run(crate::context_menu::actions::Command::EmptyTrash, state, window, cx);
                })))
        })
        .child(search::search(state, cx))
        .child(views::views(state.list_view, state.column_view, state.view_menu_open, state.preferences.foreground_alpha(), cx))
        .child(sorting::sorting(state, cx))
        .child(super::settings_button::settings_button(state.preferences.foreground_alpha(), cx))
        .into_any_element()
}
