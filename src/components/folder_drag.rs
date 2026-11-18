//! An internal folder shortcut drag, distinct from file move/copy operations.
use std::path::PathBuf;
use gpui_kit::*;
use gpui_kit::component::{ActiveTheme as _, Icon};
use gpui_kit_assets::IconName;

#[derive(Clone)]
pub struct FolderDrag {
    pub path: PathBuf,
}

impl Render for FolderDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().items_center().gap(px(10.)).px(px(14.)).py(px(10.))
            .rounded(cx.theme().radius).bg(cx.theme().popover)
            .text_color(cx.theme().popover_foreground).shadow_md()
            .child(Icon::new(IconName::Bookmark).size(px(16.)))
            .child(self.path.file_name().unwrap_or_default().to_string_lossy().into_owned())
    }
}
