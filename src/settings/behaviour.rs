use super::{controls::{Setting,Control},preferences::Preferences};
pub fn items(p: &Preferences) -> Vec<Setting> { vec![
Setting {title:"Thumbnails",description:"Preview images, GIFs, videos and documents supported by installed desktop thumbnailers.",control:Control::Toggle {checked:p.show_thumbnails,set:|p,value| p.show_thumbnails=value}},
Setting {title:"Case-sensitive search",description:"Match upper and lower case exactly when filtering the current folder.",control:Control::Toggle {checked:p.case_sensitive, set:|p,value| p.case_sensitive=value}},
] }
