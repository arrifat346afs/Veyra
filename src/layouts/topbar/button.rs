use gpui_kit::*;
use gpui_kit::component::{ActiveTheme as _, Icon};
use gpui_kit_assets::IconName;
/// Shared button styling follows the application's active palette.
pub fn button(id: &'static str, icon: IconName, label: &'static str, foreground_alpha: f32, cx: &App) -> Stateful<Div> {
    let palette = cx.theme();
    // Track foreground opacity proportionally: 40% foreground gives 60% buttons.
    let alpha = foreground_alpha.clamp(0., 1.);
    let button_alpha = (alpha * 1.5).min(1.);
    div().id(id).h(px(38.)).px(px(12.)).flex().items_center().gap(px(8.))
        .rounded(cx.theme().radius).bg(palette.muted.opacity(button_alpha))
        .border_1().border_color(palette.border.opacity(button_alpha * 0.35)).text_color(palette.foreground).cursor_pointer()
        .hover(|s| s.bg(palette.list_hover.opacity((button_alpha + 0.10).min(1.))))
        .child(Icon::new(icon).size(px(16.))).child(label)
}
