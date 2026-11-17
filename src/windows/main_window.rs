//! Main file-manager window and its navigation state.
//!
//! `MainWindow` owns the current directory and the directory entries displayed
//! beside the sidebar. Sidebar rows call `navigate_to`; file rows call the same
//! method when they represent a directory. Keeping navigation in this window
//! makes built-in places and user bookmarks behave consistently.

use std::{fs, path::PathBuf};

use gpui_kit::*;
use gpui_kit::component::ActiveTheme as _;

use crate::layouts::{content, sidebar, topbar};
use gpui_kit::component::input::{InputState, InputEvent};

pub struct MainWindow {
    pub(crate) preferences: crate::settings::preferences::Preferences,
    pub(crate) settings_open: bool,
    pub(crate) settings_page: usize,
    pub(crate) settings_search: Entity<InputState>,
    pub(crate) settings_error: Option<String>,
    focus: FocusHandle,
    pub(crate) search: Entity<InputState>,
    pub(crate) back_history: Vec<PathBuf>,
    pub(crate) forward_history: Vec<PathBuf>,
    pub(crate) list_view: bool,
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
    pub fn open(cx: &mut App) {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            window.set_window_title("Veyra");
            let (preferences, settings_error) = match crate::settings::preferences::Preferences::load() {
                Ok(p)=>(p,None), Err(e)=>(Default::default(),Some(e))
            };
            gpui_kit::component::Theme::change(if preferences.light_theme {gpui_kit::component::ThemeMode::Light} else {gpui_kit::component::ThemeMode::Dark}, Some(window), cx);
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
            let current_dir = if preferences.home_start { std::env::var_os("HOME").map(PathBuf::from).unwrap_or(current_dir) } else { current_dir };
            let (entries, error) = read_directory(&current_dir);
            cx.new(|cx| {
                let focus = cx.focus_handle();
                focus.focus(window, cx);
                let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search in Files"));
                cx.subscribe(&search, |_, _, _: &InputEvent, cx| cx.notify()).detach();
                let settings_search = cx.new(|cx| InputState::new(window,cx).placeholder("Search settings"));
                cx.subscribe(&settings_search, |_,_,_: &InputEvent,cx| cx.notify()).detach();
                Self { settings_open:false,settings_page:0,settings_search,settings_error,
                    focus, current_dir, entries, error, search, back_history: vec![], forward_history: vec![], list_view: preferences.list_view, reverse_sort: preferences.reverse_sort, show_sidebar: preferences.show_sidebar, show_hidden: preferences.show_hidden, preferences }
            })
        }).expect("Failed to open main window");
    }

    pub(crate) fn apply_preferences(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.list_view=self.preferences.list_view;
        self.reverse_sort=self.preferences.reverse_sort;
        self.show_hidden=self.preferences.show_hidden;
        self.show_sidebar=self.preferences.show_sidebar;
        gpui_kit::component::Theme::change(if self.preferences.light_theme {gpui_kit::component::ThemeMode::Light} else {gpui_kit::component::ThemeMode::Dark},Some(window),cx);
        self.settings_error=self.preferences.save().err();
        cx.notify();
    }
    /// Switch the active location and reload its visible entries.
    pub(crate) fn navigate_to(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if path != self.current_dir {
            self.back_history.push(self.current_dir.clone());
            self.forward_history.clear();
        }
        self.load_directory(path, cx);
    }

    pub(crate) fn go_back(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.back_history.pop() {
            self.forward_history.push(self.current_dir.clone());
            self.load_directory(path, cx);
        }
    }
    pub(crate) fn go_forward(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.forward_history.pop() {
            self.back_history.push(self.current_dir.clone());
            self.load_directory(path, cx);
        }
    }
    fn load_directory(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let (entries, error) = read_directory(&path);
        self.current_dir = path;
        self.entries = entries;
        self.error = error;
        cx.notify();
    }
}

/// Read a directory safely: inaccessible paths become a user-visible message.
fn read_directory(path: &PathBuf) -> (Vec<PathBuf>, Option<String>) {
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.theme().clone();
        let query = self.search.read(cx).value().to_string();
        let query = if self.preferences.case_sensitive {query} else {query.to_lowercase()};
        let mut visible: Vec<_> = self.entries.iter().filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            (self.show_hidden || !name.starts_with('.')) && (if self.preferences.case_sensitive {name.to_string()} else {name.to_lowercase()}).contains(&query)
        }).cloned().collect();
        visible.sort_by(|a,b| {
            let order = match self.preferences.sort_by {
                1=>a.metadata().map(|m|m.len()).unwrap_or(0).cmp(&b.metadata().map(|m|m.len()).unwrap_or(0)),
                2=>a.metadata().and_then(|m|m.modified()).ok().cmp(&b.metadata().and_then(|m|m.modified()).ok()),
                3=>a.extension().cmp(&b.extension()),
                _=>a.file_name().cmp(&b.file_name()),
            }.then_with(|| a.file_name().cmp(&b.file_name()));
            let order=if self.reverse_sort {order.reverse()} else {order};
            if self.preferences.folders_first {b.is_dir().cmp(&a.is_dir()).then(order)} else {order}
        });
        let toolbar = topbar::topbar(self, visible.len(), cx);
        let main_content = content::content(
            &self.current_dir,
            &visible,
            self.error.as_deref(),
            self.list_view,
            &self.preferences,
            cx,
        );
        let mut body = div().flex().flex_1().min_h(px(0.));
        if self.show_sidebar { body = body.child(sidebar::sidebar(&self.current_dir, cx)); }
        let mut root = div().relative().size_full().flex().flex_col().text_color(palette.foreground).child(toolbar).child(body.child(main_content));
        if self.settings_open { root = root.child(crate::settings::popup(self, cx)); }
        crate::keyboard_shortcuts::attach(root, &self.focus, cx)
    }
}
