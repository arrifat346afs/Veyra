use gpui_kit::*;
use gpui_kit::component::{ActiveTheme as _, Icon};
use gpui_kit_assets::IconName;
/// Shared button styling follows the application's active palette.
pub fn button(id: &'static str, icon: IconName, label: &'static str, cx: &App) -> Stateful<Div> {
    let palette = cx.theme();
    div().id(id).h(px(38.)).px(px(12.)).flex().items_center().gap(px(8.))
        .rounded(px(10.)).bg(palette.muted).text_color(palette.foreground).cursor_pointer()
        .hover(|s| s.bg(palette.list_hover))
        .child(Icon::new(icon).size(px(16.))).child(label)
}
