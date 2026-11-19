//! Compact installed-application chooser, shared by all Open with actions.
use crate::{
    services::applications::{self, Application},
    windows::MainWindow,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{
        ActiveTheme as _, Disableable, Icon,
        button::*,
        input::{Input, InputEvent, InputState},
        scroll::ScrollableElement,
    },
    *,
};
use gpui_kit_assets::IconName;
use std::path::PathBuf;

pub struct Chooser {
    paths: Vec<PathBuf>,
    applications: Vec<Application>,
    search: Entity<InputState>,
    selected: Option<String>,
    loading: bool,
    launching: bool,
    error: Option<String>,
}
impl Chooser {
    fn visible(&self, cx: &App) -> Vec<&Application> {
        let query = self.search.read(cx).value();
        self.applications
            .iter()
            .filter(|app| app.matches(&query))
            .collect()
    }
}

pub fn open(
    state: &mut MainWindow,
    paths: Vec<PathBuf>,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search applications"));
    cx.subscribe(&search, |state, _, event: &InputEvent, cx| {
        if matches!(event, InputEvent::Change) {
            if let Some(chooser) = &mut state.application_chooser {
                chooser.selected = chooser.visible(cx).first().map(|app| app.id.clone());
                chooser.error = None;
            }
            cx.notify();
        }
    })
    .detach();
    state.path_bar.cancel();
    state.application_chooser = Some(Chooser {
        paths: paths.clone(),
        applications: vec![],
        search: search.clone(),
        selected: None,
        loading: true,
        launching: false,
        error: None,
    });
    cx.defer_in(window, |state, window, cx| {
        if let Some(chooser) = &state.application_chooser {
            chooser
                .search
                .update(cx, |input, cx| input.focus(window, cx));
        }
    });
    let executor = cx.background_executor().clone();
    cx.spawn(async move |state, cx| {
        let result = executor
            .spawn(async move { applications::available_for(&paths) })
            .await;
        let _ = state.update(cx, |state, cx| {
            if let Some(chooser) = &mut state.application_chooser {
                if chooser.search != search {
                    return;
                }
                chooser.loading = false;
                match result {
                    Ok(apps) => {
                        chooser.applications = apps;
                        chooser.selected = chooser.visible(cx).first().map(|app| app.id.clone());
                    }
                    Err(error) => chooser.error = Some(error),
                }
                cx.notify();
            }
        });
    })
    .detach();
    cx.notify();
}

pub fn close(state: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    if state
        .application_chooser
        .as_ref()
        .is_some_and(|chooser| chooser.launching)
    {
        return;
    }
    state.application_chooser = None;
    state.focus.focus(window, cx);
    cx.notify();
}
fn launch(state: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let Some(chooser) = &mut state.application_chooser else {
        return;
    };
    if chooser.loading || chooser.launching {
        return;
    }
    let Some(id) = chooser.selected.clone() else {
        return;
    };
    chooser.launching = true;
    chooser.error = None;
    let paths = chooser.paths.clone();
    let recent_paths = paths.clone();
    let search = chooser.search.clone();
    let executor = cx.background_executor().clone();
    cx.spawn_in(window, async move |state, cx| {
        let result = executor
            .spawn(async move { applications::launch(&id, &paths) })
            .await;
        let _ = state.update_in(cx, |state, window, cx| {
            let Some(chooser) = &mut state.application_chooser else {
                return;
            };
            if chooser.search != search {
                return;
            }
            chooser.launching = false;
            match result {
                Ok(()) => {
                    for path in &recent_paths {
                        crate::services::navigation::record_recent(path);
                    }
                    close(state, window, cx)
                }
                Err(error) => {
                    chooser.error = Some(error);
                    cx.notify();
                }
            }
        });
    })
    .detach();
    cx.notify();
}

pub fn handle_key(
    state: &mut MainWindow,
    event: &KeyDownEvent,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) -> bool {
    let Some(chooser) = &mut state.application_chooser else {
        return false;
    };
    match event.keystroke.key.as_str() {
        "escape" => {
            close(state, window, cx);
            cx.stop_propagation();
        }
        "enter" => {
            if !event.is_held {
                launch(state, window, cx);
            }
            cx.stop_propagation();
        }
        "up" | "down" if !chooser.launching => {
            let apps = chooser.visible(cx);
            if !apps.is_empty() {
                let index = apps
                    .iter()
                    .position(|app| Some(&app.id) == chooser.selected.as_ref())
                    .unwrap_or(0);
                let index = if event.keystroke.key == "up" {
                    index.saturating_sub(1)
                } else {
                    (index + 1).min(apps.len() - 1)
                };
                chooser.selected = Some(apps[index].id.clone());
                cx.notify();
            }
            cx.stop_propagation();
        }
        _ => {}
    }
    true
}

pub fn view(state: &MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let chooser = state.application_chooser.as_ref().unwrap();
    let palette = cx.theme().clone();
    let visible = chooser.visible(cx);
    let mut list = div().flex().flex_col().gap(px(4.));
    if chooser.loading {
        list = list.child(div().p(px(12.)).child("Loading installed applications…"));
    } else if visible.is_empty() {
        list = list.child(
            div()
                .p(px(12.))
                .text_color(palette.muted_foreground)
                .child("No matching applications installed."),
        );
    }
    let mut previous_group = None;
    for app in visible {
        if previous_group != Some(app.recommended) {
            list = list.child(
                div()
                    .px(px(8.))
                    .py(px(6.))
                    .text_size(px(12.))
                    .text_color(palette.muted_foreground)
                    .child(if app.recommended {
                        "Recommended"
                    } else {
                        "Other applications"
                    }),
            );
            previous_group = Some(app.recommended);
        }
        let id = app.id.clone();
        let selected = chooser.selected.as_ref() == Some(&app.id);
        let icon = app
            .icon
            .clone()
            .map(|path| img(path).size(px(28.)).into_any_element())
            .unwrap_or_else(|| {
                Icon::new(IconName::AppWindow)
                    .size(px(28.))
                    .into_any_element()
            });
        list = list.child(
            div()
                .id(format!("choose-app-{}", app.id))
                .flex()
                .items_center()
                .gap(px(10.))
                .p(px(9.))
                .rounded(palette.radius)
                .when(selected, |row| row.bg(palette.primary.opacity(0.18)))
                .hover(|row| row.bg(palette.primary.opacity(0.12)))
                .cursor_pointer()
                .child(icon)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(div().text_ellipsis().child(app.name.clone()))
                        .when(!app.description.is_empty(), |text| {
                            text.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(palette.muted_foreground)
                                    .text_ellipsis()
                                    .child(app.description.clone()),
                            )
                        }),
                )
                .on_click(cx.listener(move |state, event: &ClickEvent, window, cx| {
                    if let Some(chooser) = &mut state.application_chooser {
                        if chooser.launching {
                            return;
                        }
                        chooser.selected = Some(id.clone());
                        chooser.error = None;
                        cx.notify();
                        if event.click_count() == 2 {
                            launch(state, window, cx);
                        }
                    }
                })),
        );
    }
    let target = if chooser.paths.len() == 1 {
        chooser.paths[0]
            .file_name()
            .unwrap_or(chooser.paths[0].as_os_str())
            .to_string_lossy()
            .into_owned()
    } else {
        format!("{} selected items", chooser.paths.len())
    };
    div()
        .absolute()
        .inset_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgba(0x00000088))
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
        .child(
            div()
                .id("application-chooser")
                .w(px(480.))
                .max_w(relative(0.94))
                .h(px(480.))
                .max_h(relative(0.88))
                .flex()
                .flex_col()
                .gap(px(12.))
                .p(px(20.))
                .rounded(palette.radius)
                .border_1()
                .border_color(palette.border)
                .bg(palette.popover)
                .text_color(palette.popover_foreground)
                .shadow_lg()
                .child(div().text_size(px(20.)).child("Open with"))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(palette.muted_foreground)
                        .text_ellipsis()
                        .child(target),
                )
                .child(Input::new(&chooser.search).disabled(chooser.launching))
                .child(
                    div()
                        .flex_1()
                        .min_h(px(0.))
                        .overflow_y_scrollbar()
                        .child(list),
                )
                .children(chooser.error.as_ref().map(|error| {
                    div()
                        .text_size(px(12.))
                        .text_color(palette.danger)
                        .child(error.clone())
                }))
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            Button::new("cancel-application")
                                .label("Cancel")
                                .disabled(chooser.launching)
                                .on_click(cx.listener(|s, _, w, c| close(s, w, c))),
                        )
                        .child(
                            Button::new("open-with-application")
                                .label(if chooser.launching {
                                    "Opening…"
                                } else {
                                    "Open"
                                })
                                .primary()
                                .disabled(
                                    chooser.loading
                                        || chooser.launching
                                        || chooser.selected.is_none(),
                                )
                                .on_click(cx.listener(|s, _, w, c| launch(s, w, c))),
                        ),
                ),
        )
        .into_any_element()
}
