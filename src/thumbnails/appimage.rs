//! Read embedded AppImage icons through 7z without running the AppImage.
use std::{io::Read, path::Path, process::{Command, Stdio}};

const MAX_ICON_BYTES: u64 = 16 * 1024 * 1024;

pub fn generate(appimage: &Path, output: &Path) -> bool {
    let Some(contents) = archive_list(appimage) else { return false; };
    let mut candidates = Vec::new();
    if contents.iter().any(|name| name == ".DirIcon") {
        candidates.push(".DirIcon".to_owned());
    }
    let app_name = appimage.file_stem().and_then(|name| name.to_str()).unwrap_or("");
    candidates.extend(contents.iter().filter(|name| {
        !name.contains('/') && is_image_name(name)
    }).cloned());
    candidates.extend(contents.iter().filter(|name| {
        name.contains("/icons/") && is_image_name(name)
            && name.to_ascii_lowercase().contains(&app_name.to_ascii_lowercase())
    }).cloned());

    for candidate in candidates {
        let Some(mut data) = extract(appimage, &candidate) else { continue; };
        for _ in 0..3 {
            if let Ok(image) = image::load_from_memory(&data) {
                return image.thumbnail(256, 256)
                    .save_with_format(output, image::ImageFormat::Png).is_ok();
            }
            if data.starts_with(b"<svg") || data.starts_with(b"<?xml") {
                if let Some(png) = render_svg(&data) {
                    if let Ok(image) = image::load_from_memory(&png) {
                        return image.thumbnail(256, 256)
                            .save_with_format(output, image::ImageFormat::Png).is_ok();
                    }
                }
            }
            let Some(target) = std::str::from_utf8(&data).ok().map(str::trim) else { break; };
            let Some(next) = icon_target(target, &contents) else { break; };
            let Some(bytes) = extract(appimage, &next) else { break; };
            data = bytes;
        }
    }
    false
}

fn is_image_name(path: &str) -> bool {
    ["png", "svg", "jpg", "jpeg", "webp"].iter().any(|ext| {
        path.rsplit_once('.').is_some_and(|(_, actual)| actual.eq_ignore_ascii_case(ext))
    })
}

fn icon_target(target: &str, contents: &[String]) -> Option<String> {
    // Symlinks inside some AppImages contain absolute build-machine paths.
    // Resolve by basename against the archive, never against the host filesystem.
    if target.is_empty() || target.contains('\0') { return None; }
    let target = target.trim_start_matches("./");
    if !target.starts_with('/') && !target.split('/').any(|part| part == "..")
        && contents.iter().any(|name| name == target) {
        return Some(target.to_owned());
    }
    let basename = Path::new(target).file_name()?.to_str()?;
    contents.iter().filter(|name| name.rsplit('/').next() == Some(basename) && is_image_name(name))
        .max_by_key(|name| icon_rank(name)).cloned()
}

fn icon_rank(path: &str) -> usize {
    if path.contains("512x512") { 512 }
    else if path.contains("256x256") { 256 }
    else if path.contains("128x128") { 128 }
    else if !path.contains('/') { 64 }
    else { 32 }
}

fn archive_list(path: &Path) -> Option<Vec<String>> {
    let bytes = run_7z(path, None)?;
    let listing = String::from_utf8(bytes).ok()?;
    Some(listing.lines().filter_map(|line| {
        let name = line.rsplit_once("  ")?.1.trim();
        (!name.is_empty() && !name.ends_with('/')).then(|| name.to_owned())
    }).collect())
}

fn extract(path: &Path, entry: &str) -> Option<Vec<u8>> {
    run_7z(path, Some(entry))
}

fn run_7z(path: &Path, entry: Option<&str>) -> Option<Vec<u8>> {
    let mut command = Command::new("7z");
    if let Some(entry) = entry { command.arg("e").arg("-so").arg(path).arg(entry); }
    else { command.arg("l").arg("-ba").arg(path); }
    let mut child = command.stdin(Stdio::null()).stdout(Stdio::piped())
        .stderr(Stdio::null()).spawn().ok()?;
    let mut bytes = Vec::new();
    let result = child.stdout.take()?.take(MAX_ICON_BYTES + 1).read_to_end(&mut bytes);
    if result.is_err() || bytes.len() as u64 > MAX_ICON_BYTES {
        let _ = child.kill(); let _ = child.wait(); return None;
    }
    child.wait().ok()?.success().then_some(bytes)
}

fn render_svg(svg: &[u8]) -> Option<Vec<u8>> {
    use std::io::Write;
    let mut child = Command::new("rsvg-convert").args(["-w", "256", "-h", "256"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    child.stdin.take()?.write_all(svg).ok()?;
    let mut data = Vec::new();
    child.stdout.take()?.take(MAX_ICON_BYTES + 1).read_to_end(&mut data).ok()?;
    if data.len() as u64 > MAX_ICON_BYTES { let _ = child.kill(); let _ = child.wait(); return None; }
    child.wait().ok()?.success().then_some(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appimage_icons_do_not_use_host_paths() {
        let contents = vec!["Descify.png".into(), "usr/share/icons/hicolor/256x256/apps/Descify.png".into()];
        assert_eq!(icon_target("/home/builder/Descify.png", &contents),
            Some("usr/share/icons/hicolor/256x256/apps/Descify.png".into()));
        assert_eq!(icon_target("../../secret", &contents), None);
    }

    #[test]
    fn extracts_a_real_appimage_icon_when_fixture_is_available() {
        let Ok(input) = std::env::var("VEYRA_APPIMAGE_TEST_PATH") else { return; };
        let output = std::env::temp_dir().join(format!("veyra-appimage-icon-{}.png", std::process::id()));
        assert!(generate(Path::new(&input), &output));
        let (width, height) = image::image_dimensions(&output).unwrap();
        assert!(width > 0 && height > 0 && width <= 256 && height <= 256);
        std::fs::remove_file(output).unwrap();
    }
}
