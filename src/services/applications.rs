//! Use GIO's desktop application registry and launcher, without changing defaults.
use gio::prelude::*;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub description: String,
    pub recommended: bool,
    pub icon: Option<PathBuf>,
}
impl Application {
    pub fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.name.to_lowercase().contains(&query)
            || self.description.to_lowercase().contains(&query)
    }
}

pub fn available_for(paths: &[PathBuf]) -> Result<Vec<Application>, String> {
    if paths.is_empty() {
        return Err("Select a file or folder first.".into());
    }
    let mut types = HashSet::new();
    for path in paths {
        let info = gio::File::for_path(path)
            .query_info(
                "standard::content-type",
                gio::FileQueryInfoFlags::NONE,
                None::<&gio::Cancellable>,
            )
            .map_err(|error| format!("{}: {error}", path.display()))?;
        types.insert(
            info.content_type()
                .map(|t| t.to_string())
                .unwrap_or_else(|| "application/octet-stream".into()),
        );
    }
    // Recommend only apps associated with every selected file type.
    let mut compatible: Option<HashSet<String>> = None;
    for content_type in types {
        let ids = gio::AppInfo::all_for_type(&content_type)
            .iter()
            .filter_map(|app| app.id().map(|id| id.to_string()))
            .collect::<HashSet<_>>();
        compatible = Some(match compatible {
            None => ids,
            Some(previous) => previous.intersection(&ids).cloned().collect(),
        });
    }
    let compatible = compatible.unwrap_or_default();
    let mut seen = HashSet::new();
    let mut applications: Vec<_> = gio::AppInfo::all()
        .into_iter()
        .filter(|app| app.should_show() && (app.supports_files() || app.supports_uris()))
        .filter_map(|app| {
            let id = app.id()?.to_string();
            if !seen.insert(id.clone()) {
                return None;
            }
            let icon = app.icon().and_then(|icon| {
                if let Some(file) = icon.downcast_ref::<gio::FileIcon>() {
                    return file.file().path();
                }
                icon.downcast_ref::<gio::ThemedIcon>()
                    .and_then(|icon| find_icon(&icon.names()))
            });
            Some(Application {
                recommended: compatible.contains(&id),
                id,
                name: app.display_name().to_string(),
                description: app.description().map(|s| s.to_string()).unwrap_or_default(),
                icon,
            })
        })
        .collect();
    applications.sort_by(|a, b| {
        b.recommended
            .cmp(&a.recommended)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then(a.id.cmp(&b.id))
    });
    Ok(applications)
}

pub fn launch(id: &str, paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("No files were selected.".into());
    }
    let app = gio::DesktopAppInfo::new(id)
        .ok_or("This application is no longer installed. Close the chooser and try again.")?;
    let files: Vec<_> = paths.iter().map(gio::File::for_path).collect();
    app.launch(&files, None::<&gio::AppLaunchContext>)
        .map_err(|error| error.to_string())
}

fn find_icon(names: &[gio::glib::GString]) -> Option<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join(".icons"));
    }
    if let Some(home) = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
    {
        roots.insert(0, home.join("icons"));
    }
    let data_dirs =
        std::env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    let data_dirs: Vec<_> = std::env::split_paths(&data_dirs).collect();
    roots.extend(data_dirs.iter().map(|dir| dir.join("icons")));
    for name in names {
        let name = name.as_str();
        if Path::new(name).is_absolute() && Path::new(name).is_file() {
            return Some(name.into());
        }
        for root in &roots {
            for theme in ["Papirus-Dark", "Papirus", "hicolor", "Adwaita"] {
                for size in ["48x48", "64x64", "scalable", "32x32"] {
                    for extension in ["svg", "png"] {
                        let path = root
                            .join(theme)
                            .join(size)
                            .join("apps")
                            .join(format!("{name}.{extension}"));
                        if path.is_file() {
                            return Some(path);
                        }
                    }
                }
            }
        }
        for dir in &data_dirs {
            for extension in ["svg", "png", "xpm"] {
                let path = dir.join("pixmaps").join(format!("{name}.{extension}"));
                if path.is_file() {
                    return Some(path);
                }
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "applications_tests.rs"]
mod tests;
