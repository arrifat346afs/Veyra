//! Local filesystem facts used by the Properties dialog.

use std::{fs, path::Path};

pub fn folder_size(path: &Path) -> Result<u64, String> {
    let mut pending = vec![path.to_path_buf()];
    let mut total = 0u64;
    while let Some(dir) = pending.pop() {
        let entries = fs::read_dir(&dir).map_err(|error| format!("{}: {error}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("{}: {error}", dir.display()))?;
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            if meta.is_dir() { pending.push(path); }
            else if meta.is_file() { total = total.saturating_add(meta.len()); }
        }
    }
    Ok(total)
}

#[cfg(unix)]
pub fn owner_group(metadata: &fs::Metadata) -> (String, String) {
    use std::os::unix::fs::MetadataExt;
    (account_name(metadata.uid(), true), account_name(metadata.gid(), false))
}

#[cfg(not(unix))]
pub fn owner_group(_: &fs::Metadata) -> (String, String) { ("Unavailable".into(), "Unavailable".into()) }

#[cfg(unix)]
fn account_name(id: u32, user: bool) -> String {
    use std::ffi::CStr;
    let mut buffer = vec![0u8; 16 * 1024];
    unsafe {
        if user {
            let mut record = std::mem::MaybeUninit::<libc::passwd>::uninit();
            let mut result = std::ptr::null_mut();
            if libc::getpwuid_r(id, record.as_mut_ptr(), buffer.as_mut_ptr().cast(), buffer.len(), &mut result) == 0 && !result.is_null() {
                let name = CStr::from_ptr((*result).pw_name).to_string_lossy().into_owned();
                if !name.is_empty() { return name; }
            }
        } else {
            let mut record = std::mem::MaybeUninit::<libc::group>::uninit();
            let mut result = std::ptr::null_mut();
            if libc::getgrgid_r(id, record.as_mut_ptr(), buffer.as_mut_ptr().cast(), buffer.len(), &mut result) == 0 && !result.is_null() {
                let name = CStr::from_ptr((*result).gr_name).to_string_lossy().into_owned();
                if !name.is_empty() { return name; }
            }
        }
    }
    id.to_string()
}

pub fn filesystem_space(path: &Path) -> Option<(String, String)> {
    use gio::prelude::*;
    let info = gio::File::for_path(path).query_filesystem_info("filesystem::type,filesystem::size,filesystem::free", None::<&gio::Cancellable>).ok()?;
    let kind = info.attribute_string("filesystem::type").map(|value| value.to_string()).unwrap_or_else(|| "Unknown".into());
    let total = info.attribute_uint64("filesystem::size");
    let free = info.attribute_uint64("filesystem::free");
    Some((kind, format!("{} total, {} free", format_bytes(total), format_bytes(free))))
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64; let mut unit = 0;
    while value >= 1024. && unit < UNITS.len() - 1 { value /= 1024.; unit += 1; }
    if unit == 0 { format!("{bytes} B") } else { format!("{value:.1} {}", UNITS[unit]) }
}

#[cfg(unix)]
pub fn toggle_permission(path: &Path, bit: u32) -> Result<(), String> {
    use std::os::unix::fs::{PermissionsExt, MetadataExt};
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    let mut mode = metadata.mode() & 0o7777;
    mode ^= bit;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|error| error.to_string())
}
