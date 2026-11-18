//! Bounded text extraction. Never extracts archive members onto the filesystem,
//! follows document links, or executes macros/embedded objects.
use std::{io::Read, path::Path};
const MAX_XML: u64 = 8 * 1024 * 1024;
const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub(super) fn paragraphs(path: &Path) -> Result<Vec<String>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Could not read this Word document: {e}"))?;
    let document = archive
        .by_name("word/document.xml")
        .map_err(|_| "This file does not contain a Word document.".to_owned())?;
    if document.size() > MAX_XML {
        return Err("This Word document is too large for a quick text preview. Open it in a document application.".into());
    }
    let mut xml = String::new();
    document
        .take(MAX_XML + 1)
        .read_to_string(&mut xml)
        .map_err(|e| e.to_string())?;
    if xml.len() as u64 > MAX_XML {
        return Err("This Word document is too large for a quick text preview.".into());
    }
    parse(&xml)
}
fn parse(xml: &str) -> Result<Vec<String>, String> {
    let document = roxmltree::Document::parse(xml)
        .map_err(|e| format!("Could not read document text: {e}"))?;
    let mut paragraphs = Vec::new();
    let mut length = 0;
    for paragraph in document
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NS, "p")))
    {
        let mut text = String::new();
        for node in paragraph.descendants().filter(|node| node.is_element()) {
            if node.tag_name().namespace() != Some(WORD_NS) {
                continue;
            }
            match node.tag_name().name() {
                "t" => text.push_str(node.text().unwrap_or_default()),
                "tab" => text.push('\t'),
                "br" | "cr" => text.push('\n'),
                _ => {}
            }
        }
        length += text.len();
        paragraphs.push(text);
        if length > 300_000 || paragraphs.len() >= 2000 {
            paragraphs.push("… Preview truncated. Open the document to read the rest.".into());
            break;
        }
    }
    if paragraphs.iter().all(|p| p.trim().is_empty()) {
        paragraphs=vec!["This document has no text to preview. Install LibreOffice for a preview of its layout and embedded images.".into()];
    }
    Ok(paragraphs)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Write};
    #[test]
    fn docx_reads_paragraphs_tables_entities_and_breaks_without_modifying_the_file() {
        let path =
            std::env::temp_dir().join(format!("veyra-word-preview-{}.docx", std::process::id()));
        let file = fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(
            "word/document.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(format!("<w:document xmlns:w=\"{WORD_NS}\"><w:body><w:p><w:r><w:t>Hello &amp; goodbye</w:t><w:tab/><w:t>World</w:t><w:br/><w:t>Next line</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>").as_bytes()).unwrap();
        zip.finish().unwrap();
        let original = fs::read(&path).unwrap();
        assert_eq!(
            paragraphs(&path).unwrap(),
            vec!["Hello & goodbye\tWorld\nNext line", "Cell"]
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn malformed_xml_and_external_entities_are_rejected() {
        assert!(parse("<broken>").is_err());
        assert!(
            parse(
                "<!DOCTYPE foo [<!ENTITY secret SYSTEM 'file:///etc/passwd'>]><foo>&secret;</foo>"
            )
            .is_err()
        );
    }
}
