use super::{controls::{Setting,Control},preferences::Preferences};
pub fn items(p: &Preferences) -> Vec<Setting> { vec![
Setting {title:"Default view",description:"Choose grid or list view.",control:Control::Choice {labels:&["Grid","List"],selected:usize::from(p.list_view),set:|p,index| p.list_view=index==1}},
Setting {title:"Show hidden files",description:"Show dotfiles by default. Ctrl+H toggles this per window.",control:Control::Toggle {checked:p.show_hidden, set:|p,value| p.show_hidden=value}},
Setting {title:"Sort by",description:"Choose the file ordering criterion.",control:Control::Choice {labels:&["Name","Size","Modified","Type"],selected:p.sort_by.min(3),set:|p,index| p.sort_by=index}},
Setting {title:"Descending order",description:"Reverse the selected sort order.",control:Control::Toggle {checked:p.reverse_sort, set:|p,value| p.reverse_sort=value}},
Setting {title:"Folders first",description:"Keep folders ahead of files in either sort direction.",control:Control::Toggle {checked:p.folders_first, set:|p,value| p.folders_first=value}},
Setting {title:"Show sidebar",description:"Display places and bookmarks.",control:Control::Toggle {checked:p.show_sidebar, set:|p,value| p.show_sidebar=value}},
Setting {title:"Start in Home",description:"Open your home folder on the next launch; otherwise use the working directory.",control:Control::Toggle {checked:p.home_start, set:|p,value| p.home_start=value}},
] }
