use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;
use crate::windows::MainWindow;
pub struct Setting { pub title: &'static str, pub description: &'static str, pub value: String, pub change: fn(&mut super::preferences::Preferences) }
pub fn rows(items: Vec<Setting>, query: &str, cx: &mut Context<MainWindow>) -> AnyElement {
    let palette = cx.theme().clone();
    if !items.iter().any(|s| format!("{} {}",s.title,s.description).to_lowercase().contains(query)) {
        return div().p(px(20.)).text_color(palette.muted_foreground).child("No settings match your search.").into_any_element();
    }
    div().flex().flex_col().gap(px(12.)).children(items.into_iter().filter(|s| format!("{} {}",s.title,s.description).to_lowercase().contains(query)).map(|s| {
        div().flex().items_center().justify_between().gap(px(24.)).p(px(18.)).rounded(px(12.)).bg(palette.muted)
            .child(div().flex_1().flex().flex_col().gap(px(6.)).child(s.title)
                .child(div().text_size(px(12.)).text_color(palette.muted_foreground).child(s.description)))
            .child(div().id(format!("setting-{}",s.title)).px(px(16.)).py(px(10.)).rounded(px(8.))
                .bg(palette.primary).text_color(palette.primary_foreground).cursor_pointer().child(s.value)
                .on_click(cx.listener(move |state,_,window,cx| {
                    (s.change)(&mut state.preferences);
                    state.apply_preferences(window,cx);
                })))
    })).into_any_element()
}
pub fn yes(value: bool) -> String { if value {"On"} else {"Off"}.into() }
