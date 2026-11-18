use gpui_kit::*;
use gpui_kit::component::{ActiveTheme as _, Icon};
use gpui_kit_assets::IconName;
/// Shared button styling follows the application's active palette.
pub fn button(id: &'static str, icon: IconName, label: &'static str, foreground_alpha: f32, cx: &App) -> Stateful<Div> {
    let palette = cx.theme();
    div().id(id).h(px(38.)).px(px(12.)).flex().items_center().gap(px(8.))
        .rounded(cx.theme().radius).bg(palette.muted.opacity(foreground_alpha)).text_color(palette.foreground).cursor_pointer()
        .hover(|s| s.bg(palette.list_hover.opacity(foreground_alpha)))
        .child(Icon::new(icon).size(px(16.))).child(label)
}
