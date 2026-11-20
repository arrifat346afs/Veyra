use std::{fs, path::Path, time::{Duration, SystemTime}};

struct Query {
    name: String,
    kind: Option<String>,
    modified_within: Option<Duration>,
    content: Option<String>,
}

fn parse(query: &str) -> Query {
    let mut name = Vec::new();
    let mut kind = None;
    let mut modified_within = None;
    let mut content = None;
    for word in query.split_whitespace() {
        if let Some(value) = word.strip_prefix("type:").filter(|v| !v.is_empty()) {
            kind = Some(value.trim_start_matches('.').to_lowercase());
        } else if let Some(value) = word.strip_prefix("modified:") {
            let (amount, unit) = value.split_at(value.find(|c: char| !c.is_ascii_digit()).unwrap_or(value.len()));
            if let Ok(amount) = amount.parse::<u64>() {
                let seconds = match unit { "d" => amount.saturating_mul(86_400), "h" => amount.saturating_mul(3_600), "m" => amount.saturating_mul(60), _ => 0 };
                if seconds > 0 { modified_within = Some(Duration::from_secs(seconds)); } else { name.push(word); }
            } else {
                name.push(word);
            }
        } else if let Some(value) = word.strip_prefix("content:").filter(|v| !v.is_empty()) {
            content = Some(value.to_string());
        } else {
            name.push(word);
        }
    }
    Query { name: name.join(" "), kind, modified_within, content }
}

pub(crate) fn filename_query(query: &str) -> String { parse(query).name }

pub(crate) fn files(root: &Path, raw_query: &str, case_sensitive: bool, show_hidden: bool) -> Vec<std::path::PathBuf> {
    let query = parse(raw_query);
    let mut pending = vec![root.to_path_buf()];
    let mut found = Vec::new();
    while let Some(directory) = pending.pop() {
        let Ok(items) = fs::read_dir(directory) else { continue };
        for item in items.flatten() {
            let path = item.path();
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            if !show_hidden && filename.starts_with('.') { continue; }
            let is_dir = item.file_type().is_ok_and(|kind| kind.is_dir());
            if is_dir { pending.push(path.clone()); }
            if matches(&path, &filename, is_dir, &query, case_sensitive) { found.push(path); }
        }
    }
    found
}

pub(crate) fn filter_entries(entries: &[std::path::PathBuf], raw_query: &str, case_sensitive: bool, show_hidden: bool) -> Vec<std::path::PathBuf> {
    let query = parse(raw_query);
    entries.iter().filter_map(|path| {
        let filename = path.file_name()?.to_string_lossy();
        if !show_hidden && filename.starts_with('.') { return None; }
        matches(path, &filename, path.is_dir(), &query, case_sensitive).then(|| path.clone())
    }).collect()
}

fn matches(path: &Path, filename: &str, is_dir: bool, query: &Query, case_sensitive: bool) -> bool {
    let candidate = if case_sensitive { filename.to_string() } else { filename.to_lowercase() };
    let name = if case_sensitive { query.name.clone() } else { query.name.to_lowercase() };
    if !candidate.contains(&name) { return false; }
    if let Some(kind) = &query.kind {
        let extension = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
        let matches = match kind.as_str() {
            "image" | "images" => mime_guess::from_path(path).first().is_some_and(|m| m.type_() == mime_guess::mime::IMAGE),
            "audio" | "music" => mime_guess::from_path(path).first().is_some_and(|m| m.type_() == mime_guess::mime::AUDIO),
            "video" => mime_guess::from_path(path).first().is_some_and(|m| m.type_() == mime_guess::mime::VIDEO),
            "document" | "documents" => mime_guess::from_path(path).first().is_some_and(|m| {
                m.type_() == mime_guess::mime::TEXT || ["pdf", "document", "opendocument", "msword", "presentation", "spreadsheet"]
                    .iter().any(|term| m.subtype().as_str().contains(term))
            }),
            other => extension == other,
        };
        if !matches { return false; }
    }
    let Ok(metadata) = fs::metadata(path) else { return false };
    if let Some(age) = query.modified_within {
        if metadata.modified().ok().and_then(|t| SystemTime::now().duration_since(t).ok()).is_none_or(|elapsed| elapsed > age) { return false; }
    }
    if let Some(needle) = &query.content {
        // shortcut: scans common text formats up to 1 MiB, use a desktop indexer for broad or large-file search.
        if is_dir || !text_file(path) || metadata.len() > 1_048_576 { return false; }
        let Ok(text) = fs::read_to_string(path) else { return false };
        let (text, needle) = if case_sensitive { (text, needle.clone()) } else { (text.to_lowercase(), needle.to_lowercase()) };
        if !text.contains(&needle) { return false; }
    }
    true
}

fn text_file(path: &Path) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str(),
        "txt" | "md" | "rst" | "csv" | "tsv" | "json" | "yaml" | "yml" | "toml" | "xml" | "html" | "css" | "js" | "ts" | "jsx" | "tsx" | "rs" | "py" | "sh" | "c" | "h" | "cpp" | "java" | "go" | "log" | "conf" | "ini")
}
