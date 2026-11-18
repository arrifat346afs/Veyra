//! A single fixed-size settings window sharing the file manager's preferences.
use crate::windows::MainWindow;
use gpui_kit::*;

const WIDTH: f32 = 900.;
const HEIGHT: f32 = 600.;

pub fn open(_state: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let owner = cx.entity().downgrade();
    // Opening a GPUI window can render its contents immediately. Do not hold
    // MainWindow's entity update while that render builds the shared panel.
    cx.defer(move |cx| open_deferred(owner, cx));
}

fn open_deferred(owner: WeakEntity<MainWindow>, cx: &mut App) {
    let Some(owner) = owner.upgrade() else {
        return;
    };
    let existing = owner.read(cx).settings_window;
    if let Some(handle) = existing {
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            return;
        }
    }
    let search = owner.read(cx).settings_search.clone();
    let window_owner = owner.clone();
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::centered(size(px(WIDTH), px(HEIGHT)), cx)),
        window_min_size: Some(size(px(WIDTH), px(HEIGHT))),
        kind: WindowKind::Floating,
        is_resizable: false,
        is_minimizable: false,
        app_id: Some("veyra-settings".into()),
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let result = cx.open_window(options, move |window, cx| {
        let owner = window_owner;
        window.set_window_title("Veyra Settings");
        let view = cx.new(|cx| {
            let changed = cx.observe(&owner, |_, _, cx| cx.notify());
            // Some Linux compositors can still send a resize despite the
            // non-resizable flag. Keep the settings client area at its fixed size.
            let bounds = cx.observe_window_bounds(window, |_, window, _| {
                let fixed = size(px(WIDTH), px(HEIGHT));
                if window.viewport_size() != fixed {
                    window.resize(fixed);
                }
            });
            let released =
                cx.observe_release_in(&owner, window, |_, _, window, _| window.remove_window());
            SettingsWindow {
                owner: owner.downgrade(),
                focus: cx.focus_handle(),
                _changed: changed,
                _released: released,
                _bounds: bounds,
            }
        });
        search.update(cx, |input, cx| input.focus(window, cx));
        cx.new(|cx| gpui_kit::base::Root::new(view, window, cx).bg(rgba(0x00000000)))
    });
    owner.update(cx, |state, cx| {
        match result {
            Ok(handle) => state.settings_window = Some(handle.into()),
            Err(error) => state.settings_error = Some(format!("Could not open settings: {error}")),
        }
        cx.notify();
    });
}

struct SettingsWindow {
    owner: WeakEntity<MainWindow>,
    focus: FocusHandle,
    _changed: Subscription,
    _released: Subscription,
    _bounds: Subscription,
}
impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(owner) = self.owner.upgrade() else {
            window.remove_window();
            return div().into_any_element();
        };
        let panel = owner.update(cx, |state, cx| super::panel::panel(state, cx));
        div()
            .id("settings-window")
            .track_focus(&self.focus)
            .size_full()
            .capture_key_down(|event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    window.remove_window();
                    cx.stop_propagation();
                }
            })
            .child(panel)
            .into_any_element()
    }
}
