//! Boolean preferences use a native switch with pointer and keyboard support.
use gpui_kit::*;
use gpui_kit::component::switch::Switch;
use crate::windows::MainWindow;
use super::preferences::Preferences;
pub fn toggle(title: &'static str, checked: bool, set: fn(&mut Preferences,bool), cx: &mut Context<MainWindow>) -> AnyElement {
    Switch::new(format!("setting-{title}")).checked(checked).accessibility_label(title)
        .on_change(cx.listener(move |state,value: &bool,window,cx| {
            set(&mut state.preferences,*value);
            state.apply_preferences(window,cx);
        })).into_any_element()
}
