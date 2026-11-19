use gpui_kit::*;
use gpui_kit::component::{ActiveTheme as _, Icon};
use gpui_kit::prelude::*;
use gpui_kit_assets::IconName;
use crate::windows::MainWindow;
use super::button::button;
pub fn views(list_view: bool, column_view: bool, menu_open: bool, foreground_alpha: f32, cx: &mut Context<MainWindow>) -> AnyElement {
    let mode = if column_view {2} else if list_view {1} else {0};
    let current_icon = [IconName::LayoutGrid, IconName::List, IconName::Columns3][mode];
    let palette = cx.theme().clone();
    let menu = deferred(div().absolute().top(px(42.)).right(px(0.)).w(px(168.)).p(px(4.))
        .rounded(cx.theme().radius).bg(palette.popover.opacity(1.)).text_color(palette.popover_foreground).shadow_md().occlude()
        .flex().flex_col().gap(px(2.))
        .children([("Grid view",IconName::LayoutGrid,0usize),("List view",IconName::List,1usize),("Column view",IconName::Columns3,2usize)]
            .into_iter().enumerate().map(|(index,(label,icon,view))| {
                div().id(format!("view-mode-{index}")).w_full().px(px(9.)).py(px(8.))
                    .rounded(cx.theme().radius).cursor_pointer().flex().items_center().gap(px(8.))
                    .when(mode == view, |row| row.bg(palette.primary.opacity(0.18)).text_color(palette.primary))
                    .hover(|style| style.bg(palette.list_hover))
                    .child(Icon::new(icon).size(px(15.))).child(label)
                    .on_click(cx.listener(move |state,_,_,cx| state.set_view_mode(view,cx)))
            }))
    ).with_priority(100);
    div().relative()
        .child(button("file-view-menu", current_icon, "", foreground_alpha, cx)
            .child(Icon::new(IconName::ChevronDown).size(px(13.)))
            .on_click(cx.listener(|state, _, _, cx| {state.view_menu_open = !state.view_menu_open;cx.notify();})))
        .when(menu_open, |root| root.child(menu))
        .into_any_element()
}
