use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;
use crate::windows::MainWindow;
pub enum Control {
    Toggle { checked: bool, set: fn(&mut super::preferences::Preferences, bool) },
    Choice { labels: &'static [&'static str], selected: usize, set: fn(&mut super::preferences::Preferences, usize) },
}
pub struct Setting { pub title: &'static str, pub description: &'static str, pub control: Control }
pub fn rows(items: Vec<Setting>, query: &str, foreground_opacity: usize, cx: &mut Context<MainWindow>) -> AnyElement {
    let palette = cx.theme().clone();
    if !items.iter().any(|s| format!("{} {}",s.title,s.description).to_lowercase().contains(query)) {
        if super::sliders::matches(query) || super::file_picker::matches(query) { return div().into_any_element(); }
        return div().p(px(20.)).text_color(palette.muted_foreground).child("No settings match your search.").into_any_element();
    }
    div().flex().flex_col().gap(px(12.)).children(items.into_iter().filter(|s| format!("{} {}",s.title,s.description).to_lowercase().contains(query)).map(|s| {
        div().flex().items_center().justify_between().gap(px(24.)).p(px(18.)).rounded(cx.theme().radius).bg(palette.muted.opacity(foreground_opacity.min(100) as f32 / 100.))
            .child(div().flex_1().flex().flex_col().gap(px(6.)).child(s.title)
                .child(div().text_size(px(12.)).text_color(palette.muted_foreground).child(s.description)))
            .child(match s.control {
                Control::Toggle {checked,set} => super::toggle::toggle(s.title,checked,set,cx),
                Control::Choice {labels,selected,set} => super::choice::choice(s.title,labels,selected,set,cx),
            })
    })).into_any_element()
}
