use gpui_kit::*;
use gpui_kit::prelude::*;
use gpui_kit::component::{ActiveTheme as _, input::Input, scroll::ScrollableElement, button::{Button, ButtonVariants}, Sizable};
use crate::windows::MainWindow;
pub fn panel(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let palette = cx.theme().clone();
    let query = state.settings_search.read(cx).value().to_lowercase();
    let mut rows = match state.settings_page {0=>super::general::items(&state.preferences),1=>super::appearance::items(&state.preferences),2=>super::behaviour::items(&state.preferences),_=>vec![]};
    if !query.is_empty() { rows = super::general::items(&state.preferences); rows.extend(super::appearance::items(&state.preferences)); rows.extend(super::behaviour::items(&state.preferences)); }
    let titles = ["General","Appearance","Behaviour","Devices","About"];
    let body = if query.is_empty() && state.settings_page>=3 {
        div().p(px(20.)).child(if state.settings_page==3 {super::devices::DESCRIPTION.to_owned()} else {super::about::description()}).into_any_element()
    } else { super::controls::rows(rows,&query,state.preferences.sidebar_opacity,cx) };
    let body = div().flex().flex_col().gap(px(12.)).child(body)
        .children((state.settings_page == 1 || !query.is_empty()).then(|| super::sliders::rows(state,&query,cx)));
    div().size_full().flex().text_color(palette.foreground)
        .child(div().w(px(200.)).flex_shrink_0().h_full().flex().flex_col().gap(px(4.)).p(px(12.)).bg(palette.muted.opacity(state.preferences.foreground_alpha()))
            .child(div().mb(px(8.)).child(Input::new(&state.settings_search)))
            .children(titles.into_iter().enumerate().map(|(i,title)| {
                let selected = state.settings_page == i && query.is_empty();
                div().id(format!("settings-page-{i}")).h(px(34.)).flex_shrink_0().flex().items_center().px(px(10.)).rounded(px(6.)).text_size(px(14.)).cursor_pointer()
                    .when(selected, |row| row.bg(palette.primary.opacity(0.16)))
                    .text_color(if selected {palette.primary} else {palette.foreground})
                    .hover(|style| style.bg(if selected {palette.primary.opacity(0.22)} else {palette.list_hover.opacity(state.preferences.foreground_alpha())}))
                    .child(title)
                    .on_click(cx.listener(move |s,_,window,cx| {s.settings_page=i;s.settings_search.update(cx, |input,cx| input.set_value("",window,cx));cx.notify();}))
            })))
        .child(div().flex_1().min_w(px(0.)).h_full().flex().flex_col().bg(palette.background.opacity(state.preferences.window_opacity.min(100) as f32 / 100.)).p(px(20.)).gap(px(20.))
            .child(div().flex().justify_between().items_center()
                .child(div().text_size(px(24.)).child(if query.is_empty() {titles[state.settings_page]} else {"Search results"}))
                .child(div().id("close-settings").cursor_pointer().px(px(14.)).py(px(8.)).rounded(px(8.)).bg(palette.muted.opacity(state.preferences.foreground_alpha())).child("Close")
                    .on_click(cx.listener(|_,_,window,_| {window.remove_window();}))))
        .children(state.settings_error.clone().or_else(||state.theme_error.clone()).map(|error| div().text_color(palette.danger_foreground).child(error)))
            .child(div().flex_1().min_h(px(0.)).overflow_y_scrollbar().child(body)
                .when(state.settings_page == 1 && query.is_empty(), |el| el.child(
                    div().mt(px(20.)).p(px(18.)).rounded(px(12.)).bg(palette.muted.opacity(state.preferences.foreground_alpha()))
                        .flex().flex_col().gap(px(10.))
                        .child("Custom theme JSON")
                        .child(div().text_size(px(12.)).text_color(palette.muted_foreground).child("Load a GPUI Kit theme or Noctalia/Matugen palette. File changes apply automatically."))
                        .child(Input::new(&state.custom_theme_path))
                        .child(Button::new("load-custom-theme").label("Load theme").small().primary().on_click(cx.listener(|s,_,window,cx| {
                            let path=s.custom_theme_path.read(cx).value().trim().to_owned();
                            match crate::settings::theme::apply(&path,cx) {
                                Ok(())=> {
                                    s.preferences.custom_theme_path=path;
                                    s.theme_error=None;
                                    s.apply_preferences(window,cx);
                                },
                                Err(error)=>s.settings_error=Some(error),
                            }
                            cx.notify();
                        })))
                ))) )
        .into_any_element()
}
