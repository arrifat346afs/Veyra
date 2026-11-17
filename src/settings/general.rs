use super::{controls::{Setting,yes},preferences::Preferences};
pub fn items(p: &Preferences) -> Vec<Setting> { vec![
Setting {title:"Default view",description:"Choose grid or list view.",value:if p.list_view {"List"} else {"Grid"}.into(),change:|p| p.list_view=!p.list_view},
Setting {title:"Show hidden files",description:"Show dotfiles by default. Ctrl+H toggles this per window.",value:yes(p.show_hidden),change:|p| p.show_hidden=!p.show_hidden},
Setting {title:"Sort by",description:"Choose the file ordering criterion.",value:["Name","Size","Modified","Type"][p.sort_by.min(3)].into(),change:|p| p.sort_by=(p.sort_by+1)%4},
Setting {title:"Descending order",description:"Reverse the selected sort order.",value:yes(p.reverse_sort),change:|p| p.reverse_sort=!p.reverse_sort},
Setting {title:"Folders first",description:"Keep folders ahead of files in either sort direction.",value:yes(p.folders_first),change:|p| p.folders_first=!p.folders_first},
Setting {title:"Show sidebar",description:"Display places and bookmarks.",value:yes(p.show_sidebar),change:|p| p.show_sidebar=!p.show_sidebar},
Setting {title:"Start in Home",description:"Open your home folder on the next launch; otherwise use the working directory.",value:yes(p.home_start),change:|p| p.home_start=!p.home_start},
] }
