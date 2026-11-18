//! Live custom-theme reloads. Poll contents rather than timestamps so atomic
//! replacements and symlink updates work too; file I/O stays off the UI thread.
use crate::windows::MainWindow;
use gpui_kit::component::Theme;
use gpui_kit::{Context, Task};
use std::{fs, rc::Rc, time::Duration};

type Contents = Result<Vec<u8>, String>;
#[derive(Default)]
struct Changes {
    observed: Option<(String, Contents)>,
}
impl Changes {
    fn changed(&mut self, path: &str, contents: &Contents) -> bool {
        if self
            .observed
            .as_ref()
            .is_some_and(|(old_path, old)| old_path == path && old == contents)
        {
            return false;
        }
        self.observed = Some((path.to_owned(), contents.clone()));
        true
    }
}
fn read(path: &str) -> Contents {
    fs::read(path).map_err(|error| format!("Could not reload theme file: {error}"))
}
fn parse(contents: Contents) -> Result<gpui_kit::component::ThemeConfig, String> {
    let bytes = contents?;
    let source =
        std::str::from_utf8(&bytes).map_err(|error| format!("Invalid theme text: {error}"))?;
    super::theme::parse(source)
}

pub fn start(cx: &mut Context<MainWindow>) -> Task<()> {
    let executor = cx.background_executor().clone();
    cx.spawn(async move |state, cx| {
        let mut changes = Changes::default();
        loop {
            executor.timer(Duration::from_millis(500)).await;
            let Ok(path) = state.update(cx, |state, _| {
                state.preferences.custom_theme_path.trim().to_owned()
            }) else {
                break;
            };
            if path.is_empty() {
                changes = Changes::default();
                continue;
            }
            let read_path = path.clone();
            let contents = executor.spawn(async move { read(&read_path) }).await;
            if !changes.changed(&path, &contents) {
                continue;
            }
            if state
                .update(cx, |state, cx| {
                    // A new file may have been selected while the old file was read.
                    if state.preferences.custom_theme_path.trim() != path {
                        return;
                    }
                    match parse(contents) {
                        Ok(config) => {
                            let config = Rc::new(config);
                            Theme::update(cx, |theme| theme.apply_config(&config));
                            state.theme_error = None;
                        }
                        // Keep the last valid palette during partial writes, missing
                        // files and invalid JSON. The next content change retries it.
                        Err(error) => state.theme_error = Some(error),
                    }
                    cx.notify();
                })
                .is_err()
            {
                break;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_edits_replacements_and_recovers_after_invalid_or_missing_files() {
        let path = std::env::temp_dir().join(format!(
            "veyra-theme-watch-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let key = path.to_str().unwrap();
        let mut changes = Changes::default();
        fs::write(&path, r##"{"primary":"#123456"}"##).unwrap();
        let first = read(key);
        assert!(changes.changed(key, &first));
        assert!(parse(first.clone()).is_ok());
        assert!(!changes.changed(key, &first));
        fs::write(&path, "{").unwrap();
        let invalid = read(key);
        assert!(changes.changed(key, &invalid));
        assert!(parse(invalid).is_err());
        let replacement = path.with_extension("tmp");
        fs::write(&replacement, r##"{"primary":"#654321"}"##).unwrap();
        fs::rename(&replacement, &path).unwrap();
        let updated = read(key);
        assert!(changes.changed(key, &updated));
        assert!(parse(updated.clone()).is_ok());
        assert!(changes.changed("different-path", &updated));
        fs::remove_file(&path).unwrap();
        let missing = read(key);
        assert!(changes.changed(key, &missing));
        assert!(parse(missing).is_err());
        fs::write(&path, r##"{"primary":"#654321"}"##).unwrap();
        let restored = read(key);
        assert!(changes.changed(key, &restored));
        assert!(parse(restored).is_ok());
        fs::remove_file(path).unwrap();
    }
}
