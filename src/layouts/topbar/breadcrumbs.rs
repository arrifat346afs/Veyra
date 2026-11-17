use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;
use crate::windows::MainWindow;
pub fn breadcrumbs(state: &MainWindow, count: usize, cx: &mut Context<MainWindow>) -> AnyElement {
    let mut path = std::path::PathBuf::new();
    let mut crumbs = Vec::new();
    for (index, part) in state.current_dir.components().enumerate() {
        path.push(part.as_os_str());
        let target = path.clone();
        let active = target == state.current_dir;
        crumbs.push(div().id(format!("crumb-{index}")).px(px(8.)).py(px(4.)).rounded(px(4.))
            .bg(if active {cx.theme().muted} else {cx.theme().background})
            .cursor_pointer().child(part.as_os_str().to_string_lossy().into_owned())
            .on_click(cx.listener(move |s, _, _, cx| s.navigate_to(target.clone(), cx))));
    }
    div().flex().items_center().justify_between().px(px(18.)).pb(px(12.)).gap(px(12.))
        .child(div().flex().flex_1().min_w(px(0.)).overflow_hidden().gap(px(4.)).children(crumbs))
        .child(div().text_size(px(12.)).text_color(cx.theme().muted_foreground).child(format!("{count} items"))).into_any_element()
}
