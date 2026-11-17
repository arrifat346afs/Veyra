use super::{controls::{Setting,yes},preferences::Preferences};
pub fn items(p: &Preferences) -> Vec<Setting> { vec![
Setting {title:"Single-click folders",description:"Turn off to open folders with a double click.",value:yes(p.single_click),change:|p| p.single_click=!p.single_click},
Setting {title:"Case-sensitive search",description:"Match upper and lower case exactly when filtering the current folder.",value:yes(p.case_sensitive),change:|p| p.case_sensitive=!p.case_sensitive},
] }
