//! Keep the visible directory in sync with filesystem changes made elsewhere.
use std::time::Duration;

use gpui_kit::{Context, Task};

use super::MainWindow;

pub fn start(cx: &mut Context<MainWindow>) -> Task<()> {
    let executor = cx.background_executor().clone();
    cx.spawn(async move |state, cx| loop {
        executor.timer(Duration::from_millis(500)).await;
        let Ok(path) = state.update(cx, |state, _| state.current_dir.clone()) else {
            break;
        };
        let read_path = path.clone();
        let (entries, error) = executor
            .spawn(async move { super::main_window::read_directory(&read_path) })
            .await;
        if state
            .update(cx, |state, cx| {
                if state.current_dir == path && (state.entries != entries || state.error != error) {
                    state.entries = entries;
                    state.error = error;
                    cx.notify();
                }
            })
            .is_err()
        {
            break;
        }
    })
}
