//! Desktop integration controls; status and installation run off the UI thread.
use crate::{
    services::portal_setup::{self, Action, Report},
    windows::MainWindow,
};
use gpui_kit::{
    component::{ActiveTheme as _, Disableable, button::*, switch::Switch},
    *,
};

#[derive(Default)]
pub struct State {
    pub report: Option<Report>,
    busy: bool,
    error: Option<String>,
    message: Option<String>,
    confirm_restart: bool,
}

pub fn matches(query: &str) -> bool {
    "use veyra for open and save dialogs file picker chooser upload desktop portal integration default application restart".contains(query)
}

pub fn run(state: &mut MainWindow, action: Action, cx: &mut Context<MainWindow>) {
    if state.portal_settings.busy {
        return;
    }
    state.portal_settings.busy = true;
    state.portal_settings.error = None;
    state.portal_settings.message = None;
    state.portal_settings.confirm_restart = false;
    cx.notify();
    let executor = cx.background_executor().clone();
    cx.spawn(async move |state, cx| {
        let result = executor
            .spawn(async move { portal_setup::perform(action) })
            .await;
        let _ = state.update(cx, |state, cx| {
            let settings = &mut state.portal_settings;
            settings.busy = false;
            match result {
                Ok((report, message)) => {
                    settings.confirm_restart =
                        report.restart_needed && matches!(action, Action::Enable | Action::Disable);
                    settings.report = Some(report);
                    settings.message = message;
                }
                Err(error) => {
                    settings.error = Some(error);
                    // A partial setup may still be recoverable. Keep its last
                    // confirmed switch state and offer Refresh for a new check.
                }
            }
            cx.notify();
        });
    })
    .detach();
}

pub fn view(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let settings = &state.portal_settings;
    let report = settings.report.as_ref();
    let enabled = report.is_some_and(|r| r.enabled);
    let unavailable = report.and_then(|r| r.unavailable.clone());
    let managed = report.is_some_and(|r| r.managed);
    let disabled = settings.busy
        || report.is_none()
        || unavailable.is_some()
        || managed
        || state.picker.is_some();
    let palette = cx.theme().clone();
    let status = if settings.busy {
        "Checking or updating file picker…".to_owned()
    } else if let Some(reason) = unavailable {
        reason
    } else if managed {
        "Your system manages the portal list. This setting cannot be changed here.".into()
    } else if let Some(report) = report {
        if report.enabled {
            if report.restart_needed {
                "Veyra selected · restart required".into()
            } else {
                "Veyra is your file picker".into()
            }
        } else if report.restart_needed {
            "Previous file picker restored · restart required".into()
        } else {
            format!("Current file picker: {}", report.current_backend)
        }
    } else {
        "Checking desktop integration…".into()
    };
    let mut card=div().id("file-picker-settings").p(px(18.)).rounded(palette.radius)
        .bg(palette.muted.opacity(state.preferences.foreground_alpha())).flex().flex_col().gap(px(12.))
        .child(div().flex().items_center().justify_between().gap(px(24.))
            .child(div().flex_1().flex().flex_col().gap(px(6.))
                .child("Use Veyra for open and save dialogs")
                .child(div().text_size(px(12.)).text_color(palette.muted_foreground)
                    .child("Choose files for uploads and other applications using Veyra. Applies to apps that use desktop portals.")))
            .child(Switch::new("use-veyra-file-picker").checked(enabled).disabled(disabled)
                .accessibility_label("Use Veyra for open and save dialogs")
                .color(palette.primary.opacity(0.30))
                .on_change(cx.listener(|state,value:&bool,_,cx|run(state,if *value {Action::Enable} else {Action::Disable},cx)))))
        .child(div().text_size(px(12.)).text_color(palette.muted_foreground).child(status));
    if let Some(report) = report {
        card = card.child(
            div()
                .text_size(px(12.))
                .text_color(palette.muted_foreground)
                .child(format!(
                    "Picker registration: {} · Automatic startup: {}",
                    if report.portal_registered {
                        "installed"
                    } else {
                        "not installed"
                    },
                    if report.service_registered {
                        "ready"
                    } else {
                        "not installed"
                    }
                )),
        );
    }
    if state.picker.is_some() {
        card = card.child(
            div()
                .text_size(px(12.))
                .child("Open settings from the main Veyra window to change your file picker."),
        );
    }
    if let Some(message) = &settings.message {
        card = card.child(div().text_size(px(12.)).child(message.clone()));
    }
    if let Some(error) = &settings.error {
        card = card.child(
            div()
                .text_size(px(12.))
                .text_color(palette.danger)
                .child(error.clone()),
        );
    }
    let mut buttons = div().flex().gap(px(8.)).child(
        Button::new("refresh-file-picker")
            .label("Refresh status")
            .ghost()
            .disabled(settings.busy)
            .on_click(cx.listener(|state, _, _, cx| run(state, Action::Refresh, cx))),
    );
    if report.is_some_and(|r| r.restart_needed) && !settings.confirm_restart {
        buttons = buttons.child(
            Button::new("restart-portal")
                .label("Apply change…")
                .ghost()
                .disabled(settings.busy || managed || state.picker.is_some())
                .on_click(cx.listener(|state, _, _, cx| {
                    state.portal_settings.confirm_restart = true;
                    cx.notify();
                })),
        );
    }
    card = card.child(buttons);
    if settings.confirm_restart {
        card=card.child(div().flex().flex_col().gap(px(8.)).pt(px(6.))
            .child("Restart the portal service?")
            .child(div().text_size(px(12.)).text_color(palette.muted_foreground)
                .child("This applies your file picker choice. Open upload/save dialogs will close, and screen sharing may be interrupted."))
            .child(div().flex().gap(px(8.))
                .child(Button::new("confirm-portal-restart").label("Restart now").ghost().disabled(settings.busy)
                    .on_click(cx.listener(|state,_,_,cx|run(state,Action::Restart,cx))))
                .child(Button::new("later-portal-restart").label("Later").ghost().disabled(settings.busy)
                    .on_click(cx.listener(|state,_,_,cx|{state.portal_settings.confirm_restart=false;cx.notify();})))));
    }
    card.into_any_element()
}
