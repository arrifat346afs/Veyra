use gpui_kit::*;
use gpui_kit::component::{ActiveTheme as _, input::Input, scroll::ScrollableElement};
use crate::windows::MainWindow;
pub fn panel(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let palette = cx.theme().clone();
    let query = state.settings_search.read(cx).value().to_lowercase();
    let mut rows = match state.settings_page {0=>super::general::items(&state.preferences),1=>super::appearance::items(&state.preferences),2=>super::behaviour::items(&state.preferences),_=>vec![]};
    if !query.is_empty() { rows = super::general::items(&state.preferences); rows.extend(super::appearance::items(&state.preferences)); rows.extend(super::behaviour::items(&state.preferences)); }
    let titles = ["General","Appearance","Behaviour","Devices","About"];
    let details = ["Views, sorting and defaults","Theme, icons and spacing","Opening folders and search","Drives and mounting","Version and capabilities"];
    let body = if query.is_empty() && state.settings_page>=3 {
        div().p(px(20.)).child(if state.settings_page==3 {super::devices::DESCRIPTION.to_owned()} else {super::about::description()}).into_any_element()
    } else { super::controls::rows(rows,&query,cx) };
    div().size_full().flex().bg(palette.background).text_color(palette.foreground)
        .child(div().w(px(220.)).flex_shrink_0().h_full().flex().flex_col().gap(px(8.)).p(px(16.)).bg(palette.muted)
            .child(Input::new(&state.settings_search))
            .children(titles.into_iter().enumerate().map(|(i,title)| {
                let selected = state.settings_page == i && query.is_empty();
                div().id(format!("settings-page-{i}")).p(px(14.)).rounded(px(10.)).cursor_pointer()
                    .bg(if selected {palette.primary} else {palette.muted})
                    .text_color(if selected {palette.primary_foreground} else {palette.foreground})
                    .hover(|style| style.bg(if selected {palette.primary} else {palette.list_hover}))
                    .child(div().child(title)).child(div().text_size(px(12.)).text_color(if selected {palette.primary_foreground} else {palette.muted_foreground}).child(details[i]))
                    .on_click(cx.listener(move |s,_,window,cx| {s.settings_page=i;s.settings_search.update(cx, |input,cx| input.set_value("",window,cx));cx.notify();}))
            })))
        .child(div().flex_1().min_w(px(0.)).h_full().flex().flex_col().p(px(20.)).gap(px(20.))
            .child(div().flex().justify_between().items_center()
                .child(div().text_size(px(24.)).child(if query.is_empty() {titles[state.settings_page]} else {"Search results"}))
                .child(div().id("close-settings").cursor_pointer().px(px(14.)).py(px(8.)).rounded(px(8.)).bg(palette.muted).child("Close")
                    .on_click(cx.listener(|s,_,window,cx| {s.settings_open=false;s.search.update(cx, |input,cx| input.focus(window,cx));cx.notify();}))))
            .children(state.settings_error.clone().map(|error| div().text_color(palette.danger_foreground).child(error)))
            .child(div().flex_1().min_h(px(0.)).overflow_y_scrollbar().child(body)))
        .into_any_element()
}
