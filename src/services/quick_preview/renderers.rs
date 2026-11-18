//! Headless document/page rendering for the native preview window.
use super::{Content, workspace};
use std::{
    fs,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Document {
    source: PathBuf,
    pub pages: u32,
    files: Arc<tempfile::TempDir>,
}
impl Document {
    pub fn render_page(&self, page: u32) -> Result<PathBuf, String> {
        if page == 0 || page > self.pages {
            return Err("Page is outside the document.".into());
        }
        let prefix = self.files.path().join(format!("page-{page}"));
        let output = prefix.with_extension("png");
        if image::image_dimensions(&output).is_ok() {
            return Ok(output);
        }
        let _ = fs::remove_file(&output);
        let executable = program("pdftoppm")
            .ok_or("Install Poppler through your software manager to preview PDF pages.")?;
        let mut command = Command::new(executable);
        command
            .args([
                "-f",
                &page.to_string(),
                "-l",
                &page.to_string(),
                "-singlefile",
                "-scale-to",
                "1800",
                "-png",
            ])
            .arg(&self.source)
            .arg(&prefix);
        run(command, Duration::from_secs(25))?;
        image::image_dimensions(&output)
            .map_err(|e| format!("Could not read rendered page: {e}"))?;
        Ok(output)
    }
}
pub(super) fn pdf(
    source: PathBuf,
    files: Arc<tempfile::TempDir>,
    description: String,
) -> Result<Content, String> {
    let executable = program("pdfinfo")
        .ok_or("Install Poppler through your software manager to preview PDF documents.")?;
    let mut command = Command::new(executable);
    command.env("LC_ALL", "C").arg(&source);
    let output = run(command, Duration::from_secs(15))?;
    let pages = page_count(&String::from_utf8_lossy(&output.stdout))
        .ok_or("Could not determine the document's page count.")?;
    let document = Document {
        source,
        pages,
        files,
    };
    let image = document.render_page(1)?;
    Ok(Content::Document {
        document,
        image,
        description,
    })
}
fn page_count(text: &str) -> Option<u32> {
    text.lines()
        .find_map(|line| {
            line.strip_prefix("Pages:")
                .and_then(|value| value.trim().parse::<u32>().ok())
        })
        .filter(|count| *count > 0)
}

pub(super) fn office(path: &Path) -> Result<Option<Content>, String> {
    let files = workspace()?;
    let mut command =
        if let Some(executable) = program("libreoffice").or_else(|| program("soffice")) {
            Command::new(executable)
        } else if program("flatpak").is_some() {
            let mut info = Command::new("flatpak");
            info.args(["info", "org.libreoffice.LibreOffice"]);
            if run(info, Duration::from_secs(5)).is_err() {
                return Ok(None);
            }
            let mut command = Command::new("flatpak");
            command
                .arg("run")
                .arg(format!("--filesystem={}", files.path().display()))
                .arg(format!(
                    "--filesystem={}:ro",
                    path.parent().unwrap().display()
                ))
                .args([
                    "--command=/app/libreoffice/program/soffice",
                    "org.libreoffice.LibreOffice",
                ]);
            command
        } else {
            return Ok(None);
        };
    // A separate profile prevents attaching to or changing a running editor.
    let profile = files.path().join("profile");
    fs::create_dir_all(profile.join("user")).map_err(|e| e.to_string())?;
    fs::write(profile.join("user/registrymodifications.xcu"),r#"<?xml version="1.0"?><oor:items xmlns:oor="http://openoffice.org/2001/registry"><item oor:path="/org.openoffice.Office.Common/Security/Scripting"><prop oor:name="MacroSecurityLevel" oor:op="fuse"><value>3</value></prop></item></oor:items>"#).map_err(|e|e.to_string())?;
    let profile_uri = url::Url::from_directory_path(&profile)
        .map_err(|_| "Could not encode preview profile path.")?;
    command
        .arg(format!("-env:UserInstallation={profile_uri}"))
        .args([
            "--headless",
            "--nologo",
            "--norestore",
            "--convert-to",
            "pdf",
            "--outdir",
        ])
        .arg(files.path())
        .arg(path);
    run(command, Duration::from_secs(60))?;
    let converted = files
        .path()
        .join(path.file_name().ok_or("Document name is missing")?)
        .with_extension("pdf");
    if !converted.is_file() {
        return Err("The document renderer could not produce a preview.".into());
    }
    pdf(converted, files, "Read-only document preview".into()).map(Some)
}
pub(super) fn video_frame(path: &Path) -> Result<Option<Content>, String> {
    let Some(executable) = program("ffmpeg") else {
        return Ok(None);
    };
    let files = workspace()?;
    let output = files.path().join("video.png");
    let mut command = Command::new(executable);
    command
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(path)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=1280:720:force_original_aspect_ratio=decrease",
            "-y",
        ])
        .arg(&output);
    if run(command, Duration::from_secs(20)).is_err() || !output.is_file() {
        return Ok(None);
    }
    Ok(Some(Content::Image {
        path: output,
        _files: files,
        description: "Video still frame · Open the file for playback".into(),
    }))
}
fn program(name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(name))
        .find(|path| {
            path.metadata().is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            })
        })
}
fn run(mut command: Command, timeout: Duration) -> Result<Output, String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|e| format!("Could not start document renderer: {e}"))?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(30)),
            other => {
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.wait();
                return Err(match other {Err(error)=>error.to_string(),_=>"The preview renderer took too long. Open the file in its application instead.".into()});
            }
        }
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "Could not render preview: {}",
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(600)
                .collect::<String>()
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_counts_are_read_without_accepting_empty_documents() {
        assert_eq!(
            page_count("Title: Test\nPages:  12\nPage size: 1x1"),
            Some(12)
        );
        assert_eq!(page_count("Pages: 0"), None);
    }
    #[test]
    fn pdf_pages_render_independently_without_changing_the_source() {
        if program("pdftoppm").is_none() || program("pdfinfo").is_none() {
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("two pages #.pdf");
        fs::write(&source, pdf_fixture()).unwrap();
        let original = fs::read(&source).unwrap();
        let content = pdf(source.clone(), workspace().unwrap(), "PDF".into()).unwrap();
        let Content::Document {
            document, image, ..
        } = content
        else {
            panic!("Expected pages");
        };
        assert_eq!(document.pages, 2);
        assert!(image.is_file());
        assert!(document.render_page(2).unwrap().is_file());
        assert!(document.render_page(3).is_err());
        assert_eq!(fs::read(source).unwrap(), original);
    }
    fn pdf_fixture() -> Vec<u8> {
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] >>",
        ];
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes.extend(format!("{} 0 obj\n{object}\nendobj\n", index + 1).bytes());
        }
        let xref = bytes.len();
        bytes.extend(b"xref\n0 5\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend(format!("{offset:010} 00000 n \n").bytes());
        }
        bytes.extend(
            format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").bytes(),
        );
        bytes
    }
}
