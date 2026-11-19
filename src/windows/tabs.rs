//! Per-window tab state and navigation commands.
use std::path::PathBuf;
use gpui_kit::{Context, Window};
use super::MainWindow;

pub(crate) struct Tabs {
    locations: Vec<PathBuf>,
    active: usize,
}

impl Tabs {
    pub(crate) fn new(initial: PathBuf) -> Self {
        Self { locations: vec![initial], active: 0 }
    }
    pub(crate) fn locations(&self) -> &[PathBuf] { &self.locations }
    pub(crate) fn active_index(&self) -> usize { self.active }
    pub(crate) fn len(&self) -> usize { self.locations.len() }
    pub(crate) fn update_location(&mut self, path: PathBuf) {
        if let Some(location) = self.locations.get_mut(self.active) { *location = path; }
    }
}

impl MainWindow {
    pub(crate) fn new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.locations.push(self.current_dir.clone());
        self.tabs.active = self.tabs.locations.len() - 1;
        cx.notify();
        self.focus.focus(window, cx);
    }

    pub(crate) fn switch_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.tabs.locations.len() || index == self.tabs.active { return; }
        self.tabs.active = index;
        let path = self.tabs.locations[index].clone();
        self.back_history.clear();
        self.forward_history.clear();
        self.load_directory(path, window, cx);
    }

    pub(crate) fn close_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.locations.len() <= 1 {
            window.remove_window();
            return;
        }
        self.tabs.locations.remove(self.tabs.active);
        self.tabs.active = self.tabs.active.min(self.tabs.locations.len() - 1);
        let path = self.tabs.locations[self.tabs.active].clone();
        self.back_history.clear();
        self.forward_history.clear();
        self.load_directory(path, window, cx);
    }
}
