use super::{controls::{Setting,yes},preferences::Preferences};
pub fn items(p: &Preferences) -> Vec<Setting> { vec![
Setting {title:"Theme",description:"Use the app's light or dark palette. System-generated colors will be added later.",value:if p.light_theme {"Light"} else {"Dark"}.into(),change:|p| p.light_theme=!p.light_theme},
Setting {title:"Icon size",description:"Papirus icon size in the grid.",value:format!("{} px",p.icon_size),change:|p| p.icon_size=match p.icon_size {64=>96,96=>112,112=>144,_=>64}},
Setting {title:"Interface density",description:"Adjust grid spacing and list row height.",value:["Spacious","Comfortable","Compact","Dense"][p.density.min(3)].into(),change:|p| p.density=(p.density+1)%4},
Setting {title:"File name size",description:"Adjust file name text size.",value:format!("{} px",p.filename_size),change:|p| p.filename_size=if p.filename_size>=16 {11} else {p.filename_size+1}},
Setting {title:"File sizes",description:"Display sizes beneath files and beside list entries.",value:yes(p.show_sizes),change:|p| p.show_sizes=!p.show_sizes},
] }
