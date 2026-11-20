//! Direct previews for common formats when no desktop thumbnailer is registered.
use std::{path::Path, process::{Command, Stdio}, time::{Duration, Instant}};

pub fn generate(mime: &str, input: &Path, output: &Path) -> bool {
    match mime {
        "application/pdf" => run(Command::new("pdftoppm")
            .args(["-f", "1", "-l", "1", "-singlefile", "-scale-to", "256", "-png"])
            .arg(input).arg(output.with_extension("")), Duration::from_secs(20)),
        "image/svg+xml" => run(Command::new("rsvg-convert")
            .args(["-w", "256", "-h", "256", "-o"]).arg(output).arg(input), Duration::from_secs(20)),
        mime if mime.starts_with("video/") => ["1", "0"].iter().any(|second| {
            run(Command::new("ffmpeg")
                .args(["-hide_banner", "-loglevel", "error", "-y", "-ss", second, "-i"])
                .arg(input).args(["-frames:v", "1", "-vf", "scale=256:256:force_original_aspect_ratio=decrease", "-update", "1"])
                .arg(output), Duration::from_secs(20))
        }),
        _ => false,
    }
}

pub(super) fn run(command: &mut Command, timeout: Duration) -> bool {
    let Ok(mut child) = command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() else { return false; };
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            _ => { let _ = child.kill(); let _ = child.wait(); return false; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_video_and_pdf_without_registered_thumbnailers() {
        if Command::new("ffmpeg").arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err()
            || Command::new("pdftoppm").arg("-v").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err() { return; }
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        assert!(Command::new("ffmpeg").args(["-y", "-loglevel", "error", "-f", "lavfi", "-i", "color=c=blue:s=320x240:d=2", "-c:v", "mpeg4"])
            .arg(&video).status().unwrap().success());
        let video_preview = dir.path().join("video.png");
        assert!(generate("video/mp4", &video, &video_preview));
        assert!(image::image_dimensions(&video_preview).is_ok_and(|(width, height)| width > 0 && height > 0 && width <= 256 && height <= 256));

        let pdf = dir.path().join("page.pdf");
        let mut body = b"%PDF-1.4\n".to_vec();
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R >>",
            "<< /Length 0 >>\nstream\n\nendstream",
        ];
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(body.len());
            body.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
        }
        let xref = body.len();
        body.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        for offset in offsets { body.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes()); }
        body.extend_from_slice(format!("trailer\n<< /Root 1 0 R /Size 5 >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
        std::fs::write(&pdf, body).unwrap();
        let pdf_preview = dir.path().join("pdf.png");
        assert!(generate("application/pdf", &pdf, &pdf_preview));
        assert!(image::image_dimensions(&pdf_preview).is_ok_and(|(width, height)| width > 0 && height > 0 && width <= 256 && height <= 256));
    }
}
