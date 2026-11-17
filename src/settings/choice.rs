//! Compact explicit choices replace buttons that cycle through hidden options.
use gpui_kit::*;
use gpui_kit::prelude::*;
use gpui_kit::component::{button::*, Sizable};
use crate::windows::MainWindow;
use super::preferences::Preferences;
pub fn choice(title: &'static str, labels: &'static [&'static str], selected: usize, set: fn(&mut Preferences,usize), cx: &mut Context<MainWindow>) -> AnyElement {
    div().flex().flex_wrap().gap(px(4.)).children(labels.iter().enumerate().map(|(index,label)| {
        Button::new(format!("setting-{title}-{index}")).label(*label).small()
            .when(index==selected, |button| button.primary())
            .on_click(cx.listener(move |state,_,window,cx| {
                set(&mut state.preferences,index);
                state.apply_preferences(window,cx);
            }))
    })).into_any_element()
}
