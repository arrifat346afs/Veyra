use super::{controls::{Setting,Control},preferences::Preferences};
pub fn items(p: &Preferences) -> Vec<Setting> { vec![
Setting {title:"Theme",description:"Choose the light, dark, or custom JSON theme.",control:Control::Choice {labels:&["Light","Dark","Custom"],selected:if p.custom_theme_enabled {2} else {usize::from(!p.light_theme)},set:|p,index| {p.custom_theme_enabled=index==2;if index<2 {p.light_theme=index==0}}}},
Setting {title:"Interface density",description:"Choose the spacing between file items.",control:Control::Choice {labels:&["Spacious","Comfortable","Compact"],selected:match p.density {0=>0,1=>1,_=>2},set:|p,index| p.density=[0,1,3][index.min(2)]}},
Setting {title:"File sizes",description:"Display sizes beneath files and beside list entries.",control:Control::Toggle {checked:p.show_sizes, set:|p,value| p.show_sizes=value}},
] }
