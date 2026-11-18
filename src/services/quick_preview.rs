//! Prepare content for Veyra's own preview window. Renderers only produce data;
//! no file-manager preview service or external viewer is launched.
mod renderers;
mod word;
pub use renderers::Document;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

pub enum Content {
    Image {
        path: PathBuf,
        _files: Arc<tempfile::TempDir>,
        description: String,
    },
    Document {
        document: Document,
        image: PathBuf,
        description: String,
    },
    Text {
        paragraphs: Vec<String>,
        description: String,
    },
    Info {
        description: String,
    },
}

pub fn prepare(path: &Path) -> Result<Content, String> {
    let resolved = path.canonicalize().map_err(|e| e.to_string())?;
    let path = resolved.as_path();
    let metadata = path.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("Select a regular file to preview.".into());
    }
    let extension = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    if extension == "pdf" {
        return renderers::pdf(path.to_owned(), workspace()?, "PDF document".into());
    }
    if matches!(
        extension.as_str(),
        "docx" | "docm" | "doc" | "odt" | "rtf" | "pptx" | "xlsx" | "ods" | "odp"
    ) {
        match renderers::office(path) {
            Ok(Some(content))=>return Ok(content),
            result if matches!(extension.as_str(),"docx"|"docm")=> {
                let description=if result.is_err() {"Text preview · Document formatting could not be rendered"} else {"Text preview · Install LibreOffice for document formatting and images"};
                return word::paragraphs(path).map(|paragraphs|Content::Text {paragraphs,description:description.into()});
            },
            Err(error)=>return Err(error),
            Ok(None)=>return Ok(Content::Info {description:"Install LibreOffice through your software manager to preview this document's pages. DOCX files also support a text preview without it.".into()}),
        }
    }
    if extension == "svg" || mime.type_() == mime_guess::mime::IMAGE {
        let files = workspace()?;
        let output = files.path().join("image.png");
        if extension == "svg" {
            if metadata.len() > 16 * 1024 * 1024 {
                return Err("This SVG is too large for Quick Preview.".into());
            }
            let output = files.path().join("image.svg");
            fs::copy(path, &output).map_err(|e| e.to_string())?;
            return Ok(Content::Image {
                path: output,
                _files: files,
                description: "SVG image".into(),
            });
        }
        let reader = image::ImageReader::open(path).map_err(|e| e.to_string())?;
        let mut reader = reader.with_guessed_format().map_err(|e| e.to_string())?;
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(256 * 1024 * 1024);
        limits.max_image_width = Some(32768);
        limits.max_image_height = Some(32768);
        reader.limits(limits);
        let mut decoder = reader
            .into_decoder()
            .map_err(|e| format!("Could not decode image: {e}"))?;
        use image::ImageDecoder;
        let dimensions = decoder.dimensions();
        let orientation = decoder
            .orientation()
            .unwrap_or(image::metadata::Orientation::NoTransforms);
        let mut image = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
        image.apply_orientation(orientation);
        image
            .thumbnail(2400, 2400)
            .save_with_format(&output, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        return Ok(Content::Image {
            path: output,
            _files: files,
            description: format!("{} × {} pixels", dimensions.0, dimensions.1),
        });
    }
    if mime.type_() == mime_guess::mime::VIDEO {
        if let Some(content) = renderers::video_frame(path)? {
            return Ok(content);
        }
    }
    // Read at most 300 KB and reject binary content. Text without an extension
    // (README, shell scripts, etc.) is supported too.
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(300_001)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if !bytes.contains(&0)
        && std::str::from_utf8(&bytes[..bytes.len().min(4096)])
            .err()
            .is_none_or(|error| error.error_len().is_none())
    {
        let truncated = bytes.len() > 300_000;
        bytes.truncate(300_000);
        let text = String::from_utf8_lossy(&bytes);
        let mut lines = text.lines();
        let mut paragraphs: Vec<_> = lines.by_ref().take(2000).map(str::to_owned).collect();
        if truncated || lines.next().is_some() {
            paragraphs.push("… Preview truncated. Open the file to read the rest.".into());
        }
        if paragraphs.is_empty() {
            paragraphs.push("This file is empty.".into());
        }
        return Ok(Content::Text {
            paragraphs,
            description: "Read-only text preview".into(),
        });
    }
    Ok(Content::Info {
        description: format!(
            "{} · {} bytes\nA content preview is not available for this file type.",
            mime.essence_str(),
            metadata.len()
        ),
    })
}
fn workspace() -> Result<Arc<tempfile::TempDir>, String> {
    tempfile::Builder::new()
        .prefix("veyra-preview-")
        .tempdir()
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn images_are_bounded_and_temporary_files_live_only_as_long_as_the_preview() {
        let root = tempfile::tempdir().unwrap();
        let input = root.path().join("image #1.png");
        image::RgbaImage::from_pixel(3000, 1200, image::Rgba([40, 90, 120, 255]))
            .save(&input)
            .unwrap();
        let original = fs::read(&input).unwrap();
        let content = prepare(&input).unwrap();
        let Content::Image {
            ref path,
            _files: ref files,
            ..
        } = content
        else {
            panic!("Expected an image");
        };
        assert_eq!(image::image_dimensions(path).unwrap(), (2400, 960));
        let generated = files.path().to_owned();
        assert!(generated.exists());
        drop(content);
        assert!(!generated.exists());
        assert_eq!(fs::read(input).unwrap(), original);
    }
    #[test]
    fn extensionless_text_and_binary_files_are_distinguished() {
        let root = tempfile::tempdir().unwrap();
        let text = root.path().join("README");
        let binary = root.path().join("binary");
        fs::write(&text, "Hello\nworld").unwrap();
        fs::write(&binary, b"\0binary").unwrap();
        assert!(matches!(prepare(&text).unwrap(), Content::Text { .. }));
        assert!(matches!(prepare(&binary).unwrap(), Content::Info { .. }));
    }
}
