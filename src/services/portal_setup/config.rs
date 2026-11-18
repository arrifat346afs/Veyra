//! Edit only the FileChooser key; retain comments and other portal choices.
pub const KEY: &str = "org.freedesktop.impl.portal.FileChooser";

pub fn value(text: &str, key: &str) -> Option<String> {
    let mut preferred = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            preferred = trimmed == "[preferred]";
        }
        if preferred && !trimmed.starts_with(['#', ';']) {
            if let Some((name, value)) = trimmed.split_once('=') {
                if name.trim() == key {
                    return Some(value.trim().to_owned());
                }
            }
        }
    }
    None
}

pub fn key_line(text: &str) -> Option<String> {
    let mut preferred = false;
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            preferred = trimmed == "[preferred]";
        }
        (preferred
            && trimmed
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == KEY))
        .then(|| line.to_owned())
    })
}

pub fn set_line(text: &str, replacement: Option<&str>) -> String {
    let mut output = Vec::new();
    let mut preferred = false;
    let mut inserted = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if preferred && !inserted {
                if let Some(replacement) = replacement {
                    output.push(replacement.to_owned());
                }
                inserted = true;
            }
            preferred = trimmed == "[preferred]";
        }
        if preferred
            && trimmed
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == KEY)
        {
            if !inserted {
                if let Some(replacement) = replacement {
                    output.push(replacement.to_owned());
                }
                inserted = true;
            }
        } else {
            output.push(line.to_owned());
        }
    }
    if !inserted {
        if let Some(replacement) = replacement {
            if !preferred {
                output.push("[preferred]".into());
            }
            output.push(replacement.into());
        }
    }
    if output.is_empty() {
        String::new()
    } else {
        output.join("\n") + "\n"
    }
}
