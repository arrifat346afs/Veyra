//! First-page previews for Office documents through LibreOffice and Poppler.
use std::{path::Path, process::Command, time::Duration};

pub fn generate(mime: &str, input: &Path, output: &Path) -> bool {
    if !supports(mime) { return false; }
    let Ok(directory) = tempfile::tempdir() else { return false; };
    let profile = directory.path().join("profile");
    let Ok(profile_uri) = url::Url::from_directory_path(&profile) else { return false; };
    let Some(name) = input.file_stem() else { return false; };
    let pdf = directory.path().join(name).with_extension("pdf");
    if !super::builtin::run(Command::new("soffice")
        .arg(format!("-env:UserInstallation={profile_uri}"))
        .args(["--headless", "--convert-to", "pdf", "--outdir"])
        .arg(directory.path()).arg(input), Duration::from_secs(60)) { return false; }
    pdf.is_file() && super::builtin::generate("application/pdf", &pdf, output)
}

fn supports(mime: &str) -> bool {
    mime.starts_with("application/vnd.openxmlformats-officedocument.")
        || mime.starts_with("application/vnd.oasis.opendocument.")
        || mime.starts_with("application/vnd.ms-")
        || matches!(mime, "application/msword" | "application/rtf" | "text/rtf")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, process::Stdio};

    #[test]
    fn renders_office_document_first_page() {
        if Command::new("soffice").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err()
            || Command::new("pdftoppm").arg("-v").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err() { return; }
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("sample.rtf");
        std::fs::write(&input, b"{\\rtf1\\ansi\\deff0 Hello from Veyra}").unwrap();
        let output = directory.path().join("preview.png");
        assert!(generate("application/rtf", &input, &output));
        assert!(image::image_dimensions(&output).is_ok_and(|(width, height)| width > 0 && height > 0 && width <= 256 && height <= 256));

        let docx = directory.path().join("sample.docx");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&docx).unwrap());
        for (name, contents) in [
            ("[Content_Types].xml", r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#),
            ("_rels/.rels", r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#),
            ("word/document.xml", r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Office thumbnail</w:t></w:r></w:p></w:body></w:document>"#),
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(contents.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        let docx_preview = directory.path().join("docx.png");
        assert!(generate("application/vnd.openxmlformats-officedocument.wordprocessingml.document", &docx, &docx_preview));
        assert!(image::image_dimensions(&docx_preview).is_ok_and(|(width, height)| width > 0 && height > 0 && width <= 256 && height <= 256));
    }
}
