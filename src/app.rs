use gpui_kit::{application, init};
use gpui_kit::component::{Theme, ThemeMode};
use crate::windows::MainWindow;

pub fn run() {
    application()
        // The default GPUI Kit bundle contains only its standard component
        // icons. This file manager uses additional Lucide icons for its places,
        // so register the full catalog to make every IconName resolvable.
        .with_assets(gpui_kit_assets::AllAssets)
        .run(|cx| {
            init(cx);
            Theme::change(ThemeMode::Dark, None, cx);
            MainWindow::open(cx);
        });
}
