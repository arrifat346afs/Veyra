//! An internal folder shortcut drag, distinct from file move/copy operations.
use std::path::PathBuf;
use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::*;

#[derive(Clone)]
pub struct FolderDrag {
    pub paths: Vec<PathBuf>,
}

impl Render for FolderDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.theme().clone();
        let previews = self.paths.iter().take(3).enumerate().map(|(index,path)| {
            div().absolute().left(px(index as f32 * 5.)).top(px(index as f32 * 5.))
                .child(crate::layouts::content::content::drag_icon(path, cx))
        });
        let count = self.paths.len();
        div().relative().w(px(48.)).h(px(48.))
            .children(previews)
            .when(count > 1, |preview| preview.child(
                div().absolute().right(px(0.)).bottom(px(0.)).min_w(px(18.)).h(px(18.))
                    .px(px(4.)).rounded_full().flex().items_center().justify_center()
                    .bg(palette.primary).text_color(palette.primary_foreground)
                    .text_size(px(11.)).shadow_sm()
                    .child(count.to_string())
            ))
    }
}
