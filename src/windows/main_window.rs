//! Main file-manager window and its navigation state.
//!
//! `MainWindow` owns the current directory and the directory entries displayed
//! beside the sidebar. Sidebar rows call `navigate_to`; file rows call the same
//! method when they represent a directory. Keeping navigation in this window
//! makes built-in places and user bookmarks behave consistently.

use std::{collections::HashMap, fs, path::PathBuf};

use gpui_kit::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::component::{ActiveTheme as _, Theme, ThemeRegistry};

use crate::layouts::{content, sidebar, topbar};
use gpui_kit::component::input::{InputState, InputEvent};

#[derive(Clone)]
pub(crate) struct FileOperationProgress {
    pub completed: usize,
    pub total: usize,
    pub current: PathBuf,
}

pub struct MainWindow {
    pub(crate) quick_preview: super::quick_preview::State,
    pub(crate) portal_settings: crate::settings::file_picker::State,
    pub(crate) picker: Option<crate::picker::Picker>,
    pub(crate) path_bar: crate::components::path_bar::PathBar,
    pub(crate) thumbnail_generation: u64,
    pub(crate) selection: crate::selection::Selection,
    pub(crate) settings_sliders: Vec<Entity<gpui_kit::component::slider::SliderState>>,
    pub(crate) preferences: crate::settings::preferences::Preferences,
    pub(crate) settings_window: Option<AnyWindowHandle>,
    pub(crate) settings_page: usize,
    pub(crate) view_menu_open: bool,
    pub(crate) settings_search: Entity<InputState>,
    pub(crate) custom_theme_path: Entity<InputState>,
    pub(crate) settings_error: Option<String>,
    pub(crate) theme_error: Option<String>,
    _theme_watch: Task<()>,
    _directory_watch: Task<()>,
    pub(crate) focus: FocusHandle,
    pub(crate) application_chooser: Option<crate::context_menu::application_chooser::Chooser>,
    pub(crate) batch_rename: Option<crate::batch_rename::BatchRename>,
    pub(crate) file_dialog: Option<crate::context_menu::dialog::Dialog>,
    pub(crate) properties_dialog: Option<PathBuf>,
    pub(crate) properties_image_details: bool,
    pub(crate) properties_metadata: Option<Vec<crate::properties::image_metadata::Section>>,
    pub(crate) properties_metadata_loading: bool,
    pub(crate) properties_metadata_error: Option<String>,
    pub(crate) properties_folder_size: Option<u64>,
    pub(crate) properties_filesystem: Option<(String, u64, u64)>,
    pub(crate) properties_error: Option<String>,
    pub(crate) file_operation_busy: bool,
    pub(crate) file_operation_progress: Option<FileOperationProgress>,
    pub(crate) undo_history: Vec<crate::context_menu::files::HistoryRecord>,
    pub(crate) redo_history: Vec<crate::context_menu::files::HistoryRecord>,
    pub(crate) cut_clipboard: Option<String>,
    pub(crate) search: Entity<InputState>,
    pub(crate) search_results: Vec<PathBuf>,
    pub(crate) search_request: u64,
    pub(crate) back_history: Vec<PathBuf>,
    pub(crate) forward_history: Vec<PathBuf>,
    pub(crate) tabs: super::tabs::Tabs,
    pub(crate) list_view: bool,
    pub(crate) column_view: bool,
    pub(crate) column_paths: Vec<PathBuf>,
    pub(crate) expanded_folders: HashMap<PathBuf, Vec<PathBuf>>,
    pub(crate) reverse_sort: bool,
    pub(crate) show_sidebar: bool,
    pub(crate) show_hidden: bool,
    /// Directory whose contents are currently visible in the main pane.
    pub(crate) current_dir: PathBuf,
    /// Snapshot of the directory contents, refreshed after each navigation.
    pub(crate) entries: Vec<PathBuf>,
    /// Last filesystem error, shown in the content area instead of hiding it.
    pub(crate) error: Option<String>,
}

impl MainWindow {
    pub fn open(picker_request: Option<crate::picker::Request>, cx: &mut App) {
        Self::open_with_location(picker_request, None, cx);
    }

    pub fn open_at(path: PathBuf, cx: &mut App) {
        Self::open_with_location(None, Some(path), cx);
    }

    fn open_with_location(picker_request: Option<crate::picker::Request>, start: Option<PathBuf>, cx: &mut App) {
        cx.open_window(WindowOptions { app_id: Some("veyra".into()), window_background: WindowBackgroundAppearance::Transparent, ..WindowOptions::default() }, move |window, cx| {
            window.set_window_title(picker_request.as_ref().map(|r| r.title.as_str()).filter(|s| !s.is_empty()).unwrap_or("Veyra"));
            if picker_request.is_some() { window.on_window_should_close(cx, |_,cx| {cx.quit(); true}); }
            let (preferences, settings_error) = match crate::settings::preferences::Preferences::load() {
                Ok(p)=>(p,None), Err(e)=>(Default::default(),Some(e))
            };
            gpui_kit::component::Theme::change(if preferences.light_theme {gpui_kit::component::ThemeMode::Light} else {gpui_kit::component::ThemeMode::Dark}, Some(window), cx);
            if preferences.custom_theme_enabled && !preferences.custom_theme_path.is_empty() {
                if let Err(error) = crate::settings::theme::apply(&preferences.custom_theme_path, cx) { eprintln!("{error}"); }
            }
            crate::settings::theme::transparent_scrollbar_track(cx);
            crate::settings::rounding::apply(preferences.corner_radius, cx);
            let current_dir = start.or_else(|| preferences.last_location.clone().filter(|path| path.is_dir())).unwrap_or_else(|| {
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
                if preferences.home_start { std::env::var_os("HOME").map(PathBuf::from).unwrap_or(cwd) } else { cwd }
            });
            let current_dir = picker_request.as_ref().map(|r| r.initial_directory()).unwrap_or(current_dir);
            let (entries, error) = read_directory(&current_dir);
            let view = cx.new(|cx| {
                let focus = cx.focus_handle();
                focus.focus(window, cx);
                let search = cx.new(|cx| InputState::new(window, cx).placeholder(""));
                cx.subscribe(&search, |_: &mut MainWindow, _, _: &InputEvent, cx| cx.notify()).detach();
                cx.subscribe_in(&search, window, |state, _, event: &InputEvent, _, cx| {
                    if !matches!(event, InputEvent::Change) { return; }
                    let query = state.search.read(cx).value().to_string();
                    let root = state.current_dir.clone();
                    let case_sensitive = state.preferences.case_sensitive;
                    let show_hidden = state.show_hidden;
                    state.search_request = state.search_request.wrapping_add(1);
                    let request = state.search_request;
                    state.search_results.clear();
                    if !query.is_empty() {
                        if root == crate::services::navigation::recent_location()
                            || root == crate::services::navigation::starred_location()
                            || crate::context_menu::files::is_trash_location(&root) {
                            state.search_results = crate::search::filter_entries(&state.entries, &query, case_sensitive, show_hidden);
                            cx.notify();
                            return;
                        }
                        let executor = cx.background_executor().clone();
                        cx.spawn(async move |state, cx| {
                            let results = executor.spawn(async move {
                                crate::search::files(&root, &query, case_sensitive, show_hidden)
                            }).await;
                            let _ = state.update(cx, |state, cx| {
                                if state.search_request == request {
                                    state.search_results = results;
                                    cx.notify();
                                }
                            });
                        }).detach();
                    }
                    cx.notify();
                }).detach();
                let settings_search = cx.new(|cx| InputState::new(window,cx).placeholder("Search settings"));
                cx.subscribe(&settings_search, |_,_,_: &InputEvent,cx| cx.notify()).detach();
                let custom_theme_path = cx.new(|cx| InputState::new(window,cx).placeholder("/path/to/theme.json"));
                custom_theme_path.update(cx, |input,cx| input.set_value(preferences.custom_theme_path.clone(),window,cx));
                cx.subscribe(&custom_theme_path, |_,_,_: &InputEvent,cx| cx.notify()).detach();
                let path_bar = crate::components::path_bar::PathBar::new(window,cx);
                let path_input = path_bar.input.clone();
                cx.subscribe_in(&path_input, window, |this,_,event: &InputEvent,window,cx| {
                    if !this.path_bar.editing {return;}
                    match event {
                        InputEvent::Change => {
                            this.path_bar.request_id = this.path_bar.request_id.wrapping_add(1);
                            let request = this.path_bar.request_id;
                            let value = this.path_bar.input.read(cx).value().to_string();
                            let current = this.current_dir.clone();
                            this.path_bar.suggestions.clear();
                            this.path_bar.highlighted = None;
                            this.path_bar.error = None;
                            let executor = cx.background_executor().clone();
                            cx.spawn(async move |this,cx| {
                                let found = executor.spawn(async move {
                                    crate::components::path_bar::suggestions::matching_folders(&value,&current)
                                }).await;
                                let _ = this.update(cx, |this,cx| {
                                    if this.path_bar.editing && this.path_bar.request_id == request {
                                        this.path_bar.suggestions = found;
                                        cx.notify();
                                    }
                                });
                            }).detach();
                            cx.notify();
                        }
                        InputEvent::PressEnter { .. } => {
                            if let Some(index) = this.path_bar.highlighted {
                                this.path_bar.accept(index,window,cx);
                                cx.notify();
                            } else {
                                match this.path_bar.resolve(&this.current_dir,cx) {
                                    Ok(path) => {this.navigate_to(path,window,cx);},
                                    Err(error) => {this.path_bar.error=Some(error);cx.notify();},
                                }
                            }
                        }
                        _ => {}
                    }
                }).detach();
                let picker = picker_request.map(|request| {
                    let filename = cx.new(|cx| InputState::new(window,cx).placeholder("File name"));
                    filename.update(cx, |input,cx| input.set_value(request.current_name.clone(),window,cx));
                    cx.subscribe_in(&filename,window,|state,_,event: &InputEvent,window,cx| {
                        if matches!(event,InputEvent::PressEnter {..}) { crate::picker::accept(state,window,cx); }
                        else { if let Some(picker) = &mut state.picker {picker.error=None;} cx.notify(); }
                    }).detach();
                    crate::picker::Picker {request,filename,error:None}
                });
                let settings_sliders = crate::settings::sliders::create(&preferences,cx);
                let theme_watch = crate::settings::theme_watch::start(cx);
                let directory_watch = super::directory_watch::start(cx);
                Self { quick_preview:Default::default(), application_chooser:None, portal_settings:Default::default(), picker, _theme_watch:theme_watch, _directory_watch:directory_watch, theme_error:None, batch_rename:None, file_dialog:None, properties_dialog:None, properties_image_details:false, properties_metadata:None, properties_metadata_loading:false, properties_metadata_error:None, properties_folder_size:None, properties_filesystem:None, properties_error:None, file_operation_busy:false, file_operation_progress:None, undo_history:vec![], redo_history:vec![], cut_clipboard:None, path_bar, thumbnail_generation: 0, selection: Default::default(), settings_sliders, settings_window:None,settings_page:0,settings_search,settings_error,
                    focus, current_dir: current_dir.clone(), entries, error, search, search_results:vec![], search_request:0, back_history: vec![], forward_history: vec![], tabs: super::tabs::Tabs::new(current_dir.clone()), list_view: preferences.list_view, column_view: preferences.column_view, column_paths: vec![current_dir], expanded_folders: HashMap::new(), view_menu_open:false, reverse_sort: preferences.reverse_sort, show_sidebar: preferences.show_sidebar, show_hidden: preferences.show_hidden, custom_theme_path, preferences }
            });
            // Override the component root’s opaque background so pane alpha reaches the compositor.
            cx.new(|cx| gpui_kit::base::Root::new(view,window,cx).bg(rgba(0x00000000)))
        }).expect("Failed to open main window");
    }

    pub(crate) fn apply_preferences(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.list_view=self.preferences.list_view;
        self.column_view=self.preferences.column_view;
        if self.column_view { self.column_paths=vec![self.current_dir.clone()]; }
        self.reverse_sort=self.preferences.reverse_sort;
        self.show_hidden=self.preferences.show_hidden;
        self.show_sidebar=self.preferences.show_sidebar;
        if !self.preferences.custom_theme_enabled {
            // Custom JSON replaces the registered theme for its mode. Restore
            // the built-in configs before asking Theme::change to reload it.
            let registry = ThemeRegistry::global(cx);
            let light = registry.default_light_theme().clone();
            let dark = registry.default_dark_theme().clone();
            Theme::update(cx, |theme| {
                theme.light_theme = light;
                theme.dark_theme = dark;
            });
            Theme::change(if self.preferences.light_theme {gpui_kit::component::ThemeMode::Light} else {gpui_kit::component::ThemeMode::Dark},Some(window),cx);
            self.theme_error=None;
        } else {
            self.theme_error=crate::settings::theme::apply(&self.preferences.custom_theme_path,cx).err();
        }
        crate::settings::theme::transparent_scrollbar_track(cx);
        crate::settings::rounding::apply(self.preferences.corner_radius, cx);
        self.settings_error=self.preferences.save().err();
        cx.notify();
    }
    pub(crate) fn set_view_mode(&mut self, mode: usize, cx: &mut Context<Self>) {
        self.list_view = mode == 1;
        self.column_view = mode == 2;
        self.preferences.list_view = self.list_view;
        self.preferences.column_view = self.column_view;
        if self.column_view { self.column_paths = vec![self.current_dir.clone()]; }
        self.view_menu_open = false;
        self.settings_error = self.preferences.save().err();
        cx.notify();
    }
    pub(crate) fn cycle_view_mode(&mut self, cx: &mut Context<Self>) {
        let current = if self.column_view {2} else if self.list_view {1} else {0};
        self.set_view_mode((current + 1) % 3, cx);
    }
    pub(crate) fn open_column(&mut self, path: PathBuf, parent_column: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.path_bar.cancel();
        self.focus.focus(window, cx);
        self.column_paths.truncate(parent_column + 1);
        self.column_paths.push(path.clone());
        let (entries, error) = read_directory(&path);
        self.current_dir = path;
        self.tabs.update_location(self.current_dir.clone());
        self.remember_location();
        self.entries = entries;
        self.error = error;
        self.selection.cursor = None;
        self.selection.anchor = None;
        self.expanded_folders.clear();
        cx.notify();
    }
    pub(crate) fn toggle_expanded_folder(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.expanded_folders.remove(&path).is_none() {
            let (entries, error) = read_directory(&path);
            if error.is_none() && !entries.is_empty() { self.expanded_folders.insert(path, entries); }
        }
        cx.notify();
    }
    /// Switch the active location and reload its visible entries.
    pub(crate) fn navigate_to(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if path != self.current_dir {
            self.back_history.push(self.current_dir.clone());
            self.forward_history.clear();
        }
        self.load_directory(path, window, cx);
    }

    pub(crate) fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.back_history.pop() {
            self.forward_history.push(self.current_dir.clone());
            self.load_directory(path, window, cx);
        }
    }
    pub(crate) fn go_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.forward_history.pop() {
            self.back_history.push(self.current_dir.clone());
            self.load_directory(path, window, cx);
        }
    }
    pub(crate) fn load_directory(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        // All navigation routes end location editing, including sidebar places,
        // bookmarks, history, and opening a folder in the content pane.
        self.path_bar.cancel();
        self.search_request = self.search_request.wrapping_add(1);
        self.search_results.clear();
        self.search.update(cx, |input,cx| input.set_value("",window,cx));
        if let Some(picker) = &mut self.picker {picker.error=None;}
        self.focus.focus(window, cx);
        let (entries, error) = read_directory(&path);
        self.selection = Default::default();
        self.expanded_folders.clear();
        self.thumbnail_generation = self.thumbnail_generation.wrapping_add(1);
        self.current_dir = path;
        self.tabs.update_location(self.current_dir.clone());
        self.remember_location();
        self.column_paths = vec![self.current_dir.clone()];
        self.entries = entries;
        self.error = error;
        cx.notify();
    }

    pub(crate) fn reconcile_entries_after_operation(&mut self) {
        if crate::context_menu::files::is_trash_location(&self.current_dir) {
            let (entries, error) = read_directory(&self.current_dir);
            self.entries = entries;
            self.error = error;
        } else {
            self.entries.retain(|entry| entry.exists());
        }
    }

    fn remember_location(&mut self) {
        if self.picker.is_none() {
            self.preferences.last_location = Some(self.current_dir.clone());
            self.settings_error = self.preferences.save().err();
        }
    }

}

/// Read a directory safely: inaccessible paths become a user-visible message.
pub(crate) fn read_directory(path: &PathBuf) -> (Vec<PathBuf>, Option<String>) {
    if path == &crate::services::navigation::recent_location() {
        return (crate::services::navigation::recent().into_iter().filter(|p| p.exists()).collect(), None);
    }
    if path == &crate::services::navigation::starred_location() {
        return (crate::services::navigation::starred().into_iter().filter(|p| p.exists()).collect(), None);
    }
    if crate::context_menu::files::is_trash_location(path) {
        return match crate::context_menu::files::read_trash() {
            Ok(entries) => (entries, None),
            Err(error) => (Vec::new(), Some(format!("Trash: {error}"))),
        };
    }
    match fs::read_dir(path) {
        Ok(items) => {
            let mut entries: Vec<_> = items.filter_map(Result::ok).map(|item| item.path()).collect();
            // Directories first, then alphabetical order, which is predictable
            // for navigation and makes the basic browser easier to scan.
            entries.sort_by_key(|entry| (!entry.is_dir(), entry.file_name().unwrap_or_default().to_string_lossy().to_lowercase()));
            (entries, None)
        }
        Err(error) => (Vec::new(), Some(format!("{}: {error}", path.display()))),
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.theme().clone();
        let query = self.search.read(cx).value().to_string();
        let name_query = crate::search::filename_query(&query);
        let query = if self.preferences.case_sensitive {name_query} else {name_query.to_lowercase()};
        let source = if self.search.read(cx).value().is_empty() {&self.entries} else {&self.search_results};
        let mut visible: Vec<_> = source.iter().filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            self.picker.as_ref().is_none_or(|p| p.request.allows(path)) && (self.show_hidden || !name.starts_with('.')) && (if self.preferences.case_sensitive {name.to_string()} else {name.to_lowercase()}).contains(&query)
        }).cloned().collect();
        let preserve_navigation_order = self.current_dir == crate::services::navigation::recent_location()
            || self.current_dir == crate::services::navigation::starred_location();
        visible.sort_by(|a,b| {
            if preserve_navigation_order {
                return self.entries.iter().position(|p| p == a).cmp(&self.entries.iter().position(|p| p == b));
            }
            let order = match self.preferences.sort_by {
                1=>a.metadata().map(|m|m.len()).unwrap_or(0).cmp(&b.metadata().map(|m|m.len()).unwrap_or(0)),
                2=>a.metadata().and_then(|m|m.modified()).ok().cmp(&b.metadata().and_then(|m|m.modified()).ok()),
                3=>a.extension().cmp(&b.extension()),
                _=>a.file_name().cmp(&b.file_name()),
            }.then_with(|| a.file_name().cmp(&b.file_name()));
            let order=if self.reverse_sort {order.reverse()} else {order};
            if self.preferences.folders_first {b.is_dir().cmp(&a.is_dir()).then(order)} else {order}
        });
        let window_width = f32::from(window.viewport_size().width);
        let toolbar = topbar::topbar(self, window, cx);
        let path_bar = crate::components::path_bar::view(self, cx);
        let selection_visible: Vec<_> = if self.list_view && !self.column_view {
            crate::layouts::content::list_tree::flatten_entries(&visible, &self.expanded_folders, self.show_hidden, &query, &self.preferences)
                .into_iter().map(|(path, _)| path).collect()
        } else { visible.clone() };
        self.selection.geometry.borrow_mut().items.clear();
        if !self.column_view { self.selection.selected.retain(|path| selection_visible.contains(path)); }
        if self.selection.cursor.as_ref().is_some_and(|path| !selection_visible.contains(path)) {
            self.selection.cursor = None;
            self.selection.anchor = None;
        }
        self.selection.visible = selection_visible;
        let main_content = content::content(
            &self.current_dir,
            &visible,
            self.error.as_deref(),
            self.list_view,
            self.column_view,
            &self.column_paths,
            self.show_hidden,
            &query,
            &self.preferences,
            &self.selection,
            &self.expanded_folders,
            self.thumbnail_generation,
            self.picker.is_some(),
            cx,
        );
        // A shared navigation frame overlays the continuous file background.
        let frame = crate::layouts::frame::Frame::default();
        let workspace = div().flex_1().min_h(px(0.)).min_w(px(0.)).flex().flex_col()
            .pr(px(12.)).pb(px(12.))
            .child(div().relative().flex_1().min_h(px(0.)).min_w(px(0.)).flex().flex_col()
                .child(frame.measure_pane())
                .child(div().flex_1().min_h(px(0.)).min_w(px(0.)).flex().flex_col()
                    .rounded(palette.radius).overflow_hidden()
                    .child(path_bar).child(main_content)
                    .child(div().id("directory-status").w_full().flex_shrink_0()
                        .flex().items_center().justify_between().px(px(12.)).py(px(6.))
                        .text_size(px(12.)).text_color(palette.muted_foreground)
                        .child(format!("{} items", visible.len()))
                        .children(crate::layouts::content::selection_status::summary(&self.selection.selected).map(|summary|
                            div().px(px(10.)).py(px(4.)).rounded(cx.theme().radius).bg(palette.popover).text_color(palette.popover_foreground).child(summary))))));
        let mut body = div().flex().size_full().min_h(px(0.)).min_w(px(0.));
        if self.show_sidebar {
            body = body.child(sidebar::sidebar(&self.current_dir, cx));
        } else {
            body = body.pl(px(12.));
        }
        let mut root = div().relative().size_full().flex().flex_col().rounded(palette.radius).overflow_hidden().text_color(palette.foreground)
            .bg(palette.muted.opacity(self.preferences.window_opacity.min(100) as f32 / 100.))
            .child(frame.background(palette.background.opacity(self.preferences.foreground_alpha()), palette.radius));
        root = root.child(body.child(div().flex_1().min_h(px(0.)).min_w(px(0.)).h_full().flex().flex_col()
            .when(self.tabs.len() > 1, |column| column.child(crate::layouts::tabs::view(self, cx)))
            .child(toolbar)
            .child(workspace)
            .children(self.picker.as_ref().map(|_| crate::picker::footer(self,cx)))));
        if self.path_bar.editing { root = root.child(crate::components::path_bar::suggestions_overlay(self,window_width,cx)); }
        if self.file_operation_busy && self.file_dialog.is_none() {
            let status = self.file_operation_progress.as_ref().map(|progress| {
                let current = progress.current.file_name().unwrap_or_default().to_string_lossy();
                format!("Working… {} of {} · {current}", progress.completed, progress.total)
            }).unwrap_or_else(|| "Working…".to_owned());
            root = root.child(div().absolute().bottom(px(16.)).right(px(16.)).p(px(12.)).rounded(cx.theme().radius).bg(palette.popover).text_color(palette.popover_foreground).child(status));
        }
        if self.file_dialog.is_some() { root = root.child(crate::context_menu::dialog::view(self,cx)); }
        if let Some(path) = self.properties_dialog.clone() { root = root.child(crate::properties::view(&path,self,cx)); }
        if self.batch_rename.is_some() { root = root.child(crate::batch_rename::view(self,cx)); }
        if self.application_chooser.is_some() {root=root.child(crate::context_menu::application_chooser::view(self,cx));}
        crate::keyboard_shortcuts::attach(root, &self.focus, cx)
    }
}
