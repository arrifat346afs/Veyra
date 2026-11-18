//! Rename template, mode selector and preview computation.
//!
//! The user picks a *mode*, types a template, and sees a live preview of the
//! resulting filenames. The dialog only commits when the user presses "Rename".
//!
//! ## Modes (identical to GNOME's Nautilus batch rename)
//!
//! | Mode              | Template meaning                                        |
//! |-------------------|---------------------------------------------------------|
//! | **Replace**       | Find text → Replace text in existing names              |
//! | **Format**        | Full new name; `[n]` = counter, `[name]` / `[ext]` =   |
//! |                   | original name / extension                               |

use std::path::{Path, PathBuf};

use gpui_kit::component::input::InputState;
use gpui_kit::*;

/// Which renaming strategy the user selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Find & replace a substring inside the original name.
    Replace,
    /// Build a new name from a template with `[n]`, `[name]`, `[ext]`.
    Format,
}

impl Mode {
    pub const ALL: [Mode; 2] = [Mode::Replace, Mode::Format];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Replace => "Find and Replace",
            Mode::Format  => "Rename using a template",
        }
    }
}

/// A single preview row.
#[derive(Clone, Debug)]
pub struct PreviewRow {
    pub original: String,
    pub renamed:  String,
    /// `true` when the resulting name is empty or conflicts with another row.
    pub error: bool,
}

/// Everything the batch-rename dialog needs.
pub struct BatchRename {
    /// The original paths being renamed (sorted).
    pub paths: Vec<PathBuf>,
    pub identities: Vec<(u64, u64)>,
    pub mode:  Mode,
    /// For Replace mode: search text.
    pub find_input:    Entity<InputState>,
    /// For Replace mode: replacement text.  For Format mode: the template.
    pub replace_input: Entity<InputState>,
    /// Starting number for the `[n]` counter (default 1).
    pub counter_start: usize,
    /// Live preview rows, recomputed after every keystroke.
    pub preview: Vec<PreviewRow>,
    /// A global error (e.g. "no matches found").
    pub error: Option<String>,
}

impl BatchRename {
    pub fn new(
        mut paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        paths.sort();
        use std::os::unix::fs::MetadataExt;
        let identities = paths.iter().map(|p| p.symlink_metadata().map(|m| (m.dev(), m.ino())).unwrap_or((0, 0))).collect();
        let find_input    = cx.new(|cx| InputState::new(window, cx).placeholder("Find"));
        let replace_input = cx.new(|cx| InputState::new(window, cx).placeholder("Replace with"));
        let preview = paths.iter().map(|p| PreviewRow {
            original: file_name(p),
            renamed:  file_name(p),
            error: false,
        }).collect();
        Self {
            paths,
            identities,
            mode: Mode::Replace,
            find_input,
            replace_input,
            counter_start: 1,
            preview,
            error: None,
        }
    }

    /// Recompute `self.preview` from the current inputs.
    pub fn refresh(&mut self, cx: &App) {
        let find    = self.find_input.read(cx).value().to_string();
        let replace = self.replace_input.read(cx).value().to_string();

        self.error = None;
        self.preview = match self.mode {
            Mode::Replace => compute_replace(&self.paths, &find, &replace),
            Mode::Format  => compute_format(&self.paths, &replace, self.counter_start),
        };

        // Check for duplicate resulting names.
        let mut seen = std::collections::HashSet::new();
        for row in &mut self.preview {
            if !seen.insert(row.renamed.clone()) {
                row.error = true;
            }
        }
        if let Some(row) = self.preview.iter().find(|r| r.error) {
            self.error = Some(if row.renamed.trim().is_empty() || row.renamed == "." || row.renamed == ".." || row.renamed.contains('/') || row.renamed.contains('\0') {
                format!("Invalid filename: {}", row.renamed)
            } else {
                format!("Two files would be renamed to {}", row.renamed)
            });
        }

        // Check for empty names.
        for row in &mut self.preview {
            if row.renamed.trim().is_empty()
                || row.renamed == "."
                || row.renamed == ".."
                || row.renamed.contains('/')
                || row.renamed.contains('\0')
            {
                row.error = true;
            }
        }
        for (path, row) in self.paths.iter().zip(&mut self.preview) {
            if path.file_name().is_some_and(|name| name.to_str().is_none()) && row.original != row.renamed {
                row.error = true;
                self.error = Some("Cannot edit a filename that is not valid UTF-8".into());
            }
        }
    }

    /// Whether any preview row has an error.
    pub fn has_errors(&self) -> bool {
        self.preview.iter().any(|r| r.error)
    }

    /// Build the list of (source, destination) rename pairs.
    pub fn rename_pairs(&self) -> Vec<(PathBuf, PathBuf)> {
        self.paths.iter().zip(self.preview.iter()).map(|(source, row)| {
            if row.original == row.renamed { return (source.clone(), source.clone()); }
            let dest = source.with_file_name(&row.renamed);
            (source.clone(), dest)
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::compute_format;
    use std::path::PathBuf;

    #[::core::prelude::v1::test]
    fn format_does_not_expand_tokens_inside_inserted_values() {
        let rows = compute_format(&[PathBuf::from("draft[ext].txt")], "[name]_copy[ext]", 1);
        assert_eq!(rows[0].renamed, "draft[ext]_copy.txt");
    }
}

// ---------------------------------------------------------------------------
// Pure computation (no GPUI types) so we can test it easily.
// ---------------------------------------------------------------------------

fn file_name(p: &Path) -> String {
    p.file_name().unwrap_or_default().to_string_lossy().to_string()
}

/// Stem without extension. `"archive.tar.gz"` → `"archive.tar"`.
fn stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(0) | None => name,
        Some(i) => &name[..i],
    }
}

/// Extension including the dot. `"a.txt"` → `".txt"`, `"Makefile"` → `""`.
fn ext(name: &str) -> &str {
    match name.rfind('.') {
        Some(0) | None => "",
        Some(i) => &name[i..],
    }
}

pub fn compute_replace(paths: &[PathBuf], find: &str, replace: &str) -> Vec<PreviewRow> {
    paths.iter().map(|p| {
        let original = file_name(p);
        let renamed = if find.is_empty() {
            original.clone()
        } else {
            original.replace(find, replace)
        };
        PreviewRow { original, renamed, error: false }
    }).collect()
}

pub fn compute_format(paths: &[PathBuf], template: &str, counter_start: usize) -> Vec<PreviewRow> {
    paths.iter().enumerate().map(|(i, p)| {
        let original = file_name(p);
        let n = counter_start + i;
        let renamed = if template.is_empty() {
            original.clone()
        } else {
            expand_template(template, n, stem(&original), ext(&original))
        };
        PreviewRow { original, renamed, error: false }
    }).collect()
}

fn expand_template(template: &str, n: usize, name: &str, ext: &str) -> String {
    let mut out = String::new();
    let mut rest = template;
    while !rest.is_empty() {
        if let Some((token, value)) = [("[00n]", format!("{:03}", n)), ("[0n]", format!("{:02}", n)), ("[n]", n.to_string()), ("[name]", name.to_string()), ("[ext]", ext.to_string())].into_iter().find(|(t, _)| rest.starts_with(t)) {
            out.push_str(&value);
            rest = &rest[token.len()..];
        } else {
            let ch = rest.chars().next().unwrap();
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }
    out
}
