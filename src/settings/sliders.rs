//! Numeric appearance preferences and their native slider controls.
use gpui_kit::*;
use gpui_kit::component::{ActiveTheme as _, slider::{Slider, SliderState, SliderEvent}};
use crate::windows::MainWindow;
use super::preferences::Preferences;
const LABELS: [&str; 5] = ["Icon size", "File name size", "Interface density", "Window opacity", "Foreground opacity"];
const DESCRIPTIONS: [&str; 5] = ["Size of Papirus icons in grid view.", "Size of file labels.", "From spacious to dense.", "Reveal the desktop through the main background; text and icons stay solid.", "Set opacity for dark-grey surfaces: sidebars and settings cards. Text and controls stay solid."];
pub fn create(p: &Preferences, cx: &mut Context<MainWindow>) -> Vec<Entity<SliderState>> {
    let specs = [(48.,192.,8.,p.icon_size),(10.,24.,1.,p.filename_size),(0.,3.,1.,p.density),(0.,100.,1.,p.window_opacity),(0.,100.,1.,p.sidebar_opacity)];
    specs.into_iter().enumerate().map(|(index,(min,max,step,value))| {
        let slider = cx.new(|_| SliderState::new().min(min).max(max).step(step).default_value((value as f32).clamp(min,max)));
        cx.subscribe(&slider, move |state,_,event: &SliderEvent,cx| {
            let (value,save) = match event { SliderEvent::Change(value)=>(value.start(),false), SliderEvent::Release(value)=>(value.start(),true) };
            let value = value.round() as usize;
            match index {0=>state.preferences.icon_size=value,1=>state.preferences.filename_size=value,2=>state.preferences.density=value,3=>state.preferences.window_opacity=value,_=>state.preferences.sidebar_opacity=value}
            if save {state.settings_error=state.preferences.save().err();}
            cx.notify();
        }).detach();
        slider
    }).collect()
}
pub fn rows(state: &MainWindow, query: &str, cx: &App) -> AnyElement {
    let palette = cx.theme();
    div().flex().flex_col().gap(px(12.)).children(state.settings_sliders.iter().enumerate().filter(|(i,_)| format!("{} {}",LABELS[*i],DESCRIPTIONS[*i]).to_lowercase().contains(query)).map(|(index,slider)| {
        let value = slider.read(cx).value().start().round() as usize;
        let label = match index {0|1=>format!("{value} px"),2=>["Spacious","Comfortable","Compact","Dense"][value.min(3)].into(),_=>format!("{value}%")};
        div().p(px(18.)).rounded(px(12.)).bg(palette.muted.opacity(state.preferences.foreground_alpha())).flex().flex_col().gap(px(12.))
            .child(div().flex().justify_between().child(LABELS[index]).child(label))
            .child(div().text_size(px(12.)).text_color(palette.muted_foreground).child(DESCRIPTIONS[index]))
            .child(Slider::new(slider).accessibility_label(LABELS[index]))
    })).into_any_element()
}

pub fn matches(query: &str) -> bool {
    LABELS.iter().zip(DESCRIPTIONS).any(|(title,description)| format!("{title} {description}").to_lowercase().contains(query))
}
