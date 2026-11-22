//! Veyra-owned Quick Preview window for every supported content type.
use super::MainWindow;
use crate::services::quick_preview::{self, Content};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{ActiveTheme as _, Disableable, button::*, scroll::ScrollableElement},
    *,
};
use std::path::PathBuf;

#[derive(Default)]
pub struct State {
    window: Option<AnyWindowHandle>,
    path: Option<PathBuf>,
    opening: bool,
}

pub fn toggle(state: &mut MainWindow, _window: &mut Window, cx: &mut Context<MainWindow>) {
    if state.quick_preview.opening || state.selection.selected.len() != 1 {
        return;
    }
    let path = state.selection.selected.iter().next().unwrap().clone();
    if !path.is_file() {
        return;
    }
    if quick_preview::show_with_sushi(&path) {
        return;
    }
    if let Some(handle) = state.quick_preview.window.take() {
        let closed = handle
            .update(cx, |_, window, _| window.remove_window())
            .is_ok();
        if closed && state.quick_preview.path.as_ref() == Some(&path) {
            state.quick_preview.path = None;
            return;
        }
    }
    state.quick_preview.opening = true;
    let owner = cx.entity().downgrade();
    // Opening can render immediately; let the main entity update finish first.
    cx.defer(move |cx| {
        let Some(preview_owner) = owner.upgrade() else {
            return;
        };
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(900.), px(700.)), cx)),
            window_min_size: Some(size(px(440.), px(340.))),
            kind: WindowKind::Floating,
            app_id: Some("veyra-preview".into()),
            ..Default::default()
        };
        let preview_path = path.clone();
        let result = cx.open_window(options, move |window, cx| {
            window.set_window_title(&format!(
                "{} — Veyra Preview",
                preview_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            ));
            let view = cx.new(|cx| {
                let focus = cx.focus_handle();
                focus.focus(window, cx);
                let released = cx.observe_release_in(&preview_owner, window, |_, _, window, _| {
                    window.remove_window()
                });
                let mut preview = QuickPreview {
                    path: preview_path,
                    content: None,
                    page_image: None,
                    page: 1,
                    zoom: 1.,
                    busy: true,
                    error: None,
                    focus,
                    _released: released,
                    _task: None,
                };
                preview.load(cx);
                preview
            });
            cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
        });
        let _ = owner.update(cx, |state, cx| {
            state.quick_preview.opening = false;
            match result {
                Ok(handle) => {
                    state.quick_preview.window = Some(handle.into());
                    state.quick_preview.path = Some(path);
                }
                Err(error) => state.error = Some(format!("Could not open preview: {error}")),
            }
            cx.notify();
        });
    });
}
struct QuickPreview {
    path: PathBuf,
    content: Option<Content>,
    page_image: Option<PathBuf>,
    page: u32,
    zoom: f32,
    busy: bool,
    error: Option<String>,
    focus: FocusHandle,
    _released: Subscription,
    _task: Option<Task<()>>,
}
impl QuickPreview {
    fn load(&mut self, cx: &mut Context<Self>) {
        let path = self.path.clone();
        let executor = cx.background_executor().clone();
        self._task = Some(cx.spawn(async move |view, cx| {
            let result = executor
                .spawn(async move { quick_preview::prepare(&path) })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(content) => {
                        view.page_image = match &content {
                            Content::Image { path, .. } => Some(path.clone()),
                            Content::Video { path, .. } => Some(path.clone()),
                            Content::Document { image, .. } => Some(image.clone()),
                            _ => None,
                        };
                        view.content = Some(content);
                    }
                    Err(error) => view.error = Some(error),
                }
                cx.notify();
            });
        }));
    }
    fn page(&mut self, page: u32, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(Content::Document { document, .. }) = &self.content else {
            return;
        };
        if page == 0 || page > document.pages || page == self.page {
            return;
        }
        let document = document.clone();
        let executor = cx.background_executor().clone();
        self.busy = true;
        self.error = None;
        cx.notify();
        self._task = Some(cx.spawn(async move |view, cx| {
            let result = executor
                .spawn(async move { document.render_page(page) })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(image) => {
                        view.page_image = Some(image);
                        view.page = page;
                    }
                    Err(error) => view.error = Some(error),
                }
                cx.notify();
            });
        }));
    }
    fn zoom(&mut self, factor: f32, cx: &mut Context<Self>) {
        self.zoom = (self.zoom * factor).clamp(0.25, 4.);
        cx.notify();
    }
}
/// Use the same raised foreground surface as the file manager's controls.
fn preview_button(id: &'static str, cx: &App) -> Button {
    let palette = cx.theme();
    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(palette.muted)
                .foreground(palette.foreground)
                .hover(palette.list_hover)
                .active(palette.muted)
                .shadow(false),
        )
        .border_0()
        .rounded(palette.radius)
}

impl Render for QuickPreview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.theme().clone();
        let path = self.path.clone();
        let pages = match &self.content {
            Some(Content::Document { document, .. }) => document.pages,
            _ => 1,
        };
        let description = match &self.content {
            Some(Content::Image { description, .. })
            | Some(Content::Video { description, .. })
            | Some(Content::Audio { description })
            | Some(Content::Document { description, .. })
            | Some(Content::Text { description, .. }) => description.as_str(),
            _ => "",
        };
        let body = if let Some(Content::Audio { description }) = &self.content {
            let play_path = self.path.clone();
            div().flex_1().flex().flex_col().items_center().justify_center().gap(px(16.))
                .child(div().text_size(px(15.)).text_color(palette.muted_foreground).child(description.clone()))
                .child(preview_button("preview-play-audio", cx).label("Play audio")
                    .on_click(move |_, _, cx| cx.open_with_system(&play_path)))
                .into_any_element()
        } else if let Some(image) = &self.page_image {
            let width = (f32::from(window.viewport_size().width) - 40.).max(200.);
            let height = (f32::from(window.viewport_size().height) - 124.).max(140.);
            let image_width = width * self.zoom;
            let image_height = height * self.zoom;
            div()
                .id("preview-image-scroll")
                .px(px(20.))
                .flex_1()
                .min_h(px(0.))
                .min_w(px(0.))
                .overflow_scroll()
                .child(
                    div()
                        .w(px(image_width.max(width)))
                        .h(px(image_height.max(height)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            img(image.clone())
                                .w(px(image_width))
                                .h(px(image_height))
                                .flex_shrink_0()
                                .object_fit(ObjectFit::Contain)
                                .with_fallback(|| {
                                    div()
                                        .p(px(20.))
                                        .child("Could not display this image.")
                                        .into_any_element()
                                }),
                        ),
                )
                .into_any_element()
        } else if let Some(Content::Text { paragraphs, .. }) = &self.content {
            div()
                .flex_1()
                .min_h(px(0.))
                .min_w(px(0.))
                .overflow_y_scrollbar()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .p(px(28.))
                        .text_size(px(16.))
                        .children(
                            paragraphs
                                .iter()
                                .map(|paragraph| div().min_h(px(12.)).child(paragraph.clone())),
                        ),
                )
                .into_any_element()
        } else {
            div().flex_1().min_h(px(0.)).flex().items_center().justify_center().p(px(28.))
                .child(if self.busy {"Loading preview…".to_owned()} else if let Some(Content::Info {description})=&self.content {description.clone()} else {"A preview could not be displayed. You can open the original file using the button above.".into()}).into_any_element()
        };
        div()
            .id("veyra-quick-preview")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.background)
            .text_color(palette.foreground)
            .capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers != Modifiers::default() {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "space" | "escape" => {
                        if !event.is_held {
                            window.remove_window();
                        }
                        cx.stop_propagation();
                    }
                    "left" => {
                        view.page(view.page.saturating_sub(1), cx);
                        cx.stop_propagation();
                    }
                    "right" => {
                        view.page(view.page.saturating_add(1), cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(20.))
                    .py(px(14.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .child(
                                div().text_size(px(14.)).text_ellipsis().child(
                                    self.path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                ),
                            )
                            .when(!description.is_empty(), |title| {
                                title.child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(palette.muted_foreground)
                                        .text_ellipsis()
                                        .child(description.to_owned()),
                                )
                            }),
                    )
                    .child(
                        preview_button("open-preview-original", cx)
                            .label("Open file")
                            .on_click(move |_, _, cx| cx.open_with_system(&path)),
                    )
                    .when(matches!(self.content.as_ref(), Some(Content::Video { .. })), |header| {
                        let play_path = self.path.clone();
                        header.child(preview_button("preview-play-video", cx).label("Play video")
                            .on_click(move |_, _, cx| cx.open_with_system(&play_path)))
                    })
                    .child(
                        preview_button("close-preview", cx)
                            .label("Close")
                            .on_click(|_, window, _| window.remove_window()),
                    ),
            )
            .children(self.error.as_ref().map(|error| {
                div()
                    .flex_shrink_0()
                    .p(px(12.))
                    .text_size(px(13.))
                    .text_color(palette.danger)
                    .child(error.clone())
            }))
            .child(body)
            .when(self.page_image.is_some() || self.busy, |view| {
                view.child(
                    div()
                        .flex_shrink_0()
                        .h(px(54.))
                        .px(px(20.))
                        .pb(px(12.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .when(
                            matches!(self.content, Some(Content::Document { .. })),
                            |bar| {
                                bar.child(
                                    preview_button("preview-previous-page", cx)
                                        .label("Previous")
                                        .disabled(self.page <= 1 || self.busy)
                                        .on_click(cx.listener(|view, _, _, cx| {
                                            view.page(view.page.saturating_sub(1), cx)
                                        })),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .child(format!("Page {} of {}", self.page, pages)),
                                )
                                .child(
                                    preview_button("preview-next-page", cx)
                                        .label("Next")
                                        .disabled(self.page >= pages || self.busy)
                                        .on_click(cx.listener(|view, _, _, cx| {
                                            view.page(view.page + 1, cx)
                                        })),
                                )
                            },
                        )
                        .child(div().flex_1())
                        .when(self.page_image.is_some(), |bar| {
                            bar.child(
                                preview_button("preview-zoom-out", cx)
                                    .label("−")
                                    .disabled(self.zoom <= 0.25)
                                    .on_click(
                                        cx.listener(|view, _, _, cx| view.zoom(1. / 1.25, cx)),
                                    ),
                            )
                            .child(
                                preview_button("preview-fit", cx)
                                    .label(format!("{}% · Fit", (self.zoom * 100.).round() as u32))
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.zoom = 1.;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                preview_button("preview-zoom-in", cx)
                                    .label("+")
                                    .disabled(self.zoom >= 4.)
                                    .on_click(cx.listener(|view, _, _, cx| view.zoom(1.25, cx))),
                            )
                        })
                        .when(self.busy, |bar| {
                            bar.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(palette.muted_foreground)
                                    .child("Loading…"),
                            )
                        }),
                )
            })
    }
}
