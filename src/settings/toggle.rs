//! Boolean preferences use a native switch with pointer and keyboard support.
use gpui_kit::*;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::ActiveTheme as _;
use crate::windows::MainWindow;
use super::preferences::Preferences;
pub fn toggle(title: &'static str, checked: bool, set: fn(&mut Preferences,bool), cx: &mut Context<MainWindow>) -> AnyElement {
    Switch::new(format!("setting-{title}")).checked(checked).accessibility_label(title)
        // Keep the track softer than the solid thumb so the enabled switch
        // retains its shape even when primary and secondary share a color.
        .color(cx.theme().primary.opacity(0.30))
        .on_change(cx.listener(move |state,value: &bool,window,cx| {
            set(&mut state.preferences,*value);
            state.apply_preferences(window,cx);
        })).into_any_element()
}
