//! File chooser shared by the standalone picker and the desktop portal backend.
pub mod portal;
mod view;
pub use view::{accept, footer, handle_key};

use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub type Filter = (String, Vec<(u32, String)>);
pub type Choice = (String, String, Vec<(String, String)>, String);

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Request {
    pub title: String,
    pub accept_label: String,
    pub multiple: bool,
    pub directory: bool,
    pub save: bool,
    pub current_folder: Option<PathBuf>,
    pub current_name: String,
    pub filters: Vec<Filter>,
    pub filter_index: usize,
    pub choices: Vec<Choice>,
    /// SaveFiles asks for a destination directory and returns one URI per name.
    pub files: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Reply {
    pub uris: Vec<String>,
    pub current_filter: Option<Filter>,
    pub choices: Vec<(String, String)>,
}

pub struct Picker {
    pub request: Request,
    pub filename: gpui_kit::Entity<gpui_kit::component::input::InputState>,
    pub error: Option<String>,
}

impl Request {
    pub fn initial_directory(&self) -> PathBuf {
        self.current_folder
            .clone()
            .filter(|p| p.is_dir())
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("/"))
    }

    pub fn allows(&self, path: &Path) -> bool {
        if path.is_dir() {
            return true;
        }
        if self.directory {
            return false;
        }
        let Some((_, patterns)) = self.filters.get(self.filter_index) else {
            return true;
        };
        patterns.iter().any(|(kind, pattern)| match kind {
            0 => globset::Glob::new(pattern).ok().is_some_and(|glob| {
                glob.compile_matcher()
                    .is_match(Path::new(path.file_name().unwrap_or_default()))
            }),
            1 => mime_guess::from_path(path).iter().any(|mime| {
                mime.essence_str() == pattern
                    || pattern
                        .strip_suffix("/*")
                        .is_some_and(|prefix| mime.type_().as_str() == prefix)
            }),
            _ => false,
        })
    }

    pub fn selected_paths(
        &self,
        directory: &Path,
        selected: &HashSet<PathBuf>,
        name: &str,
    ) -> Result<Vec<PathBuf>, String> {
        if self.save {
            validate_name(name)?;
            let path = directory.join(name);
            if path.is_dir() {
                return Err("Choose a file name, not a folder.".into());
            }
            return Ok(vec![path]);
        }
        let mut paths: Vec<_> = selected.iter().cloned().collect();
        paths.sort();
        if self.directory && paths.is_empty() {
            paths.push(directory.to_owned());
        }
        if paths.is_empty() {
            return Err("Select a file to continue.".into());
        }
        if !self.multiple && paths.len() != 1 {
            return Err("Select just one item.".into());
        }
        for path in &paths {
            if if self.directory {
                !path.is_dir()
            } else {
                !path.is_file() || !self.allows(path)
            } {
                return Err(if self.directory {
                    "Select a folder."
                } else {
                    "Select a file matching the selected file type."
                }
                .into());
            }
        }
        if !self.files.is_empty() {
            let mut reserved = HashSet::new();
            let mut targets = Vec::new();
            for name in &self.files {
                validate_name(name)?;
                let original = paths[0].join(name);
                let mut path = original.clone();
                let mut suffix = 1;
                while path.exists() || reserved.contains(&path) {
                    let stem = original.file_stem().unwrap_or_default().to_string_lossy();
                    let extension = original
                        .extension()
                        .map(|e| format!(".{}", e.to_string_lossy()))
                        .unwrap_or_default();
                    path = original.with_file_name(format!("{stem} ({suffix}){extension}"));
                    suffix += 1;
                }
                reserved.insert(path.clone());
                targets.push(path);
            }
            return Ok(targets);
        }
        Ok(paths)
    }

    pub fn reply(&self, paths: &[PathBuf]) -> Result<Reply, String> {
        let uris = paths
            .iter()
            .map(|path| {
                url::Url::from_file_path(path)
                    .map(|uri| uri.to_string())
                    .map_err(|_| "Could not encode the selected path.".to_owned())
            })
            .collect::<Result<_, _>>()?;
        Ok(Reply {
            uris,
            current_filter: self.filters.get(self.filter_index).cloned(),
            choices: self
                .choices
                .iter()
                .map(|(id, _, _, value)| (id.clone(), value.clone()))
                .collect(),
        })
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\0']) {
        Err("Enter a valid file name without slashes.".into())
    } else {
        Ok(())
    }
}

/// Accept writes exactly one JSON response; closing/cancelling writes nothing.
pub fn finish(request: &Request, paths: &[PathBuf], cx: &mut gpui_kit::App) -> Result<(), String> {
    use std::io::Write;
    let reply = request.reply(paths)?;
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &reply).map_err(|e| e.to_string())?;
    stdout
        .write_all(b"\n")
        .and_then(|_| stdout.flush())
        .map_err(|e| e.to_string())?;
    cx.quit();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filters_keep_navigation_and_support_globs_and_mime_families() {
        let mut request = Request {
            filters: vec![(
                "Images".into(),
                vec![(0, "*.png".into()), (1, "image/*".into())],
            )],
            ..Default::default()
        };
        assert!(request.allows(Path::new("/")));
        assert!(request.allows(Path::new("photo.jpg")));
        assert!(request.allows(Path::new("photo.png")));
        assert!(!request.allows(Path::new("report.pdf")));
        request.directory = true;
        assert!(!request.allows(Path::new("photo.png")));
    }
    #[test]
    fn selection_validates_cardinality_type_and_save_names() {
        let root = std::env::temp_dir().join(format!("veyra-picker-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let a = root.join("a #.txt");
        let b = root.join("b.txt");
        std::fs::write(&a, "a").unwrap();
        std::fs::write(&b, "b").unwrap();
        let mut request = Request::default();
        let selected = HashSet::from([a.clone(), b]);
        assert!(request.selected_paths(&root, &selected, "").is_err());
        request.multiple = true;
        assert_eq!(
            request.selected_paths(&root, &selected, "").unwrap().len(),
            2
        );
        assert!(request.reply(&[a]).unwrap().uris[0].contains("a%20%23.txt"));
        request.save = true;
        for bad in ["", "..", "../escape", "/absolute", "a\0b"] {
            assert!(request.selected_paths(&root, &HashSet::new(), bad).is_err());
        }
        request.save = false;
        request.directory = true;
        request.multiple = false;
        assert_eq!(
            request.selected_paths(&root, &HashSet::new(), "").unwrap(),
            vec![root.clone()]
        );
        request.files = vec!["a #.txt".into(), "a #.txt".into()];
        let paths = request.selected_paths(&root, &HashSet::new(), "").unwrap();
        assert_ne!(paths[0], paths[1]);
        assert!(paths.iter().all(|p| !p.exists()));
        std::fs::remove_dir_all(root).unwrap();
    }
}
