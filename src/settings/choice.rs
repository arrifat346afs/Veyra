//! Compact explicit choices replace buttons that cycle through hidden options.
use gpui_kit::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::{button::*, Sizable};
use crate::windows::MainWindow;
use super::preferences::Preferences;
pub fn choice(title: &'static str, labels: &'static [&'static str], selected: usize, set: fn(&mut Preferences,usize), cx: &mut Context<MainWindow>) -> AnyElement {
    let palette = cx.theme().clone();
    div().flex().flex_wrap().gap(px(4.)).children(labels.iter().enumerate().map(|(index,label)| {
        let active = index == selected;
        Button::new(format!("setting-{title}-{index}"))
            .label(if active {format!("✓ {label}")} else {(*label).to_owned()})
            .small()
            .when(active, |button| button.primary())
            .when(!active, |button| button.custom(ButtonCustomVariant::new(cx)
                .color(palette.muted.opacity(0.65))
                .foreground(palette.foreground)
                .hover(palette.muted)
                .active(palette.muted)))
            .on_click(cx.listener(move |state,_,window,cx| {
                set(&mut state.preferences,index);
                state.apply_preferences(window,cx);
            }))
    })).into_any_element()
}
