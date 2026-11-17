use super::{controls::{Setting,Control},preferences::Preferences};
pub fn items(p: &Preferences) -> Vec<Setting> { vec![
Setting {title:"Theme",description:"Use the app's light or dark palette. System-generated colors will be added later.",control:Control::Choice {labels:&["Light","Dark"],selected:usize::from(!p.light_theme),set:|p,index| p.light_theme=index==0}},
Setting {title:"File sizes",description:"Display sizes beneath files and beside list entries.",control:Control::Toggle {checked:p.show_sizes, set:|p,value| p.show_sizes=value}},
] }
