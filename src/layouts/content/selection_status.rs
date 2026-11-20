//! Compact selection details shown in the content footer.

use std::{collections::HashSet, fs, path::PathBuf};

pub fn summary(selected: &HashSet<PathBuf>) -> Option<String> {
    if selected.is_empty() { return None; }
    let files: Vec<_> = selected.iter().filter_map(|path| {
        let metadata = fs::symlink_metadata(path).ok()?;
        (!metadata.is_dir()).then_some((path, metadata.len()))
    }).collect();
    let size = (!files.is_empty() && files.len() == selected.len()).then(|| format_size(files.iter().map(|(_, size)| size).sum()));
    if selected.len() == 1 {
        let name = selected.iter().next().unwrap().file_name().unwrap_or_default().to_string_lossy();
        Some(match size { Some(size) => format!("“{name}” selected ({size})"), None => format!("“{name}” selected") })
    } else {
        Some(match size { Some(size) => format!("{} items selected ({size})", selected.len()), None => format!("{} items selected", selected.len()) })
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000. && unit < UNITS.len() - 1 { value /= 1000.; unit += 1; }
    if unit == 0 { format!("{bytes} B") } else { format!("{value:.1} {}", UNITS[unit]) }
}
