//! Discover freedesktop thumbnail providers installed by the desktop and user.
use std::{collections::HashSet, ffi::OsString, fs, path::{Path, PathBuf}, process::{Command, Stdio}, sync::OnceLock, time::{Duration, Instant}};
pub struct Provider { command: Vec<String>, mime_types: Vec<String> }
pub fn providers() -> &'static [Provider] {
    static PROVIDERS: OnceLock<Vec<Provider>> = OnceLock::new();
    PROVIDERS.get_or_init(|| {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        let mut dirs = vec![std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"))];
        dirs.extend(std::env::split_paths(&std::env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into())));
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for dir in dirs {
            let mut files: Vec<_> = fs::read_dir(dir.join("thumbnailers")).into_iter().flatten().flatten().map(|e| e.path()).collect();
            files.sort();
            for path in files {
                if path.extension().and_then(|x| x.to_str()) != Some("thumbnailer") || !seen.insert(path.file_name().unwrap().to_owned()) {continue;}
                if let Ok(text) = fs::read_to_string(path) {if let Some(provider) = parse(&text) {result.push(provider);}}
            }
        }
        result
    })
}
fn parse(text: &str) -> Option<Provider> {
    let mut section = false;
    let mut exec = None;
    let mut mime_types = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {section = line == "[Thumbnailer Entry]";continue;}
        if !section {continue;}
        if let Some(value) = line.strip_prefix("Exec=") {exec = shell_words::split(value).ok();}
        if let Some(value) = line.strip_prefix("MimeType=") {mime_types = value.split(';').filter(|v| !v.is_empty()).map(str::to_owned).collect();}
    }
    let command: Vec<String> = exec?;
    if command.is_empty() {return None;}
    Some(Provider {command,mime_types})
}
impl Provider {
    pub fn supports(&self, mime: &str) -> bool {self.mime_types.iter().any(|m| m == mime)}
    pub fn generate(&self, input: &Path, output: &Path) -> bool {
        let Some(uri) = url::Url::from_file_path(input).ok() else {return false;};
        let mut args = Vec::<OsString>::new();
        for arg in &self.command[1..] {
            args.push(match arg.as_str() {
                "%i" => input.as_os_str().to_owned(), "%o" => output.as_os_str().to_owned(),
                "%u" => uri.as_str().into(), "%s" => "256".into(), "%%" => "%".into(),
                _ if arg.contains('%') => return false,
                _ => arg.into(),
            });
        }
        // Execute argv directly: filenames and provider arguments never enter a shell.
        let Ok(mut child) = Command::new(&self.command[0]).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() else {return false;};
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return status.success(),
                Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
                _ => {let _ = child.kill();let _ = child.wait();return false;}
            }
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn parses_registered_provider_without_shell_expansion() {
        let p=super::parse("[Thumbnailer Entry]\nExec=\"/usr/bin/my thumbnailer\" -i %i -o %o -s %s\nMimeType=image/gif;video/mp4;\n").unwrap();
        assert!(p.supports("image/gif"));
        assert!(!p.supports("text/plain"));
        assert_eq!(p.command[0], "/usr/bin/my thumbnailer");
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    #[test]
    fn installed_video_provider_generates_a_real_preview() {
        if Command::new("ffmpeg").arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err()
            || Command::new("ffmpegthumbnailer").arg("-v").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err() {return;}
        let dir=std::env::temp_dir().join(format!("veyra-video-test-{}",std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let input=dir.join("video ' with spaces.mp4");
        let output=dir.join("preview.png");
        assert!(Command::new("ffmpeg").args(["-y","-f","lavfi","-i","color=c=blue:s=320x240:d=1","-c:v","mpeg4"])
            .arg(&input).stdout(Stdio::null()).stderr(Stdio::null()).status().unwrap().success());
        let provider=providers().iter().find(|p| p.command[0]=="ffmpegthumbnailer" && p.supports("video/mp4")).unwrap();
        assert!(provider.generate(&input,&output));
        let (width,height)=image::image_dimensions(&output).unwrap();
        assert!(width>0 && height>0 && width<=256 && height<=256);
        fs::remove_dir_all(dir).unwrap();
    }
}
