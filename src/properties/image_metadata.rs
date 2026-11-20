//! Read descriptive and technical embedded tags with ExifTool.

use std::{path::Path, process::Command};

#[derive(Clone)]
pub struct Section {
    pub title: &'static str,
    pub fields: Vec<(&'static str, String)>,
}

const FIELDS: &[(&str, &str, &[&str])] = &[
    ("Description", "Title", &["title", "objectname", "imagedescription", "xptitle"]),
    ("Description", "Description", &["description", "caption-abstract", "usercomment", "xpcomment"]),
    ("Description", "Keywords", &["keywords", "subject", "xpkeywords", "hierarchicalsubject", "category"]),
    ("Description", "Creator", &["creator", "artist", "author", "by-line", "creatorworkemail"]),
    ("Description", "Copyright", &["copyright", "rights"]),
    ("Description", "Location", &["location"]),
    ("Description", "Rating", &["rating", "starrating"]),
    ("Image", "Megapixels", &["megapixels"]),
    ("Image", "Color Space", &["colorspace", "profiledescription"]),
    ("Image", "Bit Depth", &["bitdepth", "bitsper sample"]),
    ("Camera", "Date Taken", &["datetimeoriginal", "createdate", "datecreated"]),
    ("Camera", "Camera Make", &["make"]),
    ("Camera", "Camera Model", &["model"]),
    ("Camera", "Lens", &["lensmodel", "lensid"]),
    ("Camera", "Exposure", &["exposuretime"]),
    ("Camera", "Aperture", &["fnumber", "aperture"]),
    ("Camera", "ISO", &["iso"]),
    ("Camera", "Focal Length", &["focallength"]),
    ("Camera", "Flash", &["flash"]),
    ("Camera", "Metering Mode", &["meteringmode"]),
    ("Camera", "Exposure Mode", &["exposuremode", "exposureprogram"]),
    ("Camera", "Software", &["software"]),
    ("Camera", "GPS Latitude", &["gpslatitude"]),
    ("Camera", "GPS Longitude", &["gpslongitude"]),
    ("Document", "Page Count", &["pagecount", "pages"]),
    ("Document", "Word Count", &["wordcount"]),
    ("Document", "Producer", &["producer"]),
    ("Media", "Duration", &["duration"]),
    ("Media", "Artist", &["albumartist", "performer"]),
    ("Media", "Album", &["album"]),
    ("Media", "Genre", &["genre"]),
    ("Media", "Video Codec", &["videocodec"]),
    ("Media", "Audio Codec", &["audiocodec"]),
    ("Media", "Bitrate", &["averagebitrate", "audio bitrate"]),
];

pub fn read(path: &Path) -> Result<Vec<Section>, String> {
    let mut command = Command::new("exiftool");
    command.args(["-json", "-G1", "-s", "-Title", "-Description", "-Keywords", "-Subject", "-ObjectName", "-ImageDescription", "-XPTitle", "-XPComment", "-XPKeywords", "-XPSubject", "-Caption-Abstract", "-Headline", "-HierarchicalSubject", "-Creator", "-Artist", "-By-line", "-Copyright", "-Rights", "-Rating", "-Location",
        "-ImageWidth", "-ImageHeight", "-Megapixels", "-ColorSpace", "-BitDepth", "-DateTimeOriginal", "-CreateDate", "-Make", "-Model",
        "-DateCreated", "-LensModel", "-ExposureTime", "-FNumber", "-ISO", "-FocalLength", "-GPSLatitude", "-GPSLongitude", "-ExposureProgram", "-ExposureMode", "-Flash", "-MeteringMode", "-Software", "-PageCount", "-WordCount",
        "-Producer", "-Duration", "-AlbumArtist", "-Performer", "-Album", "-Genre", "-VideoCodec", "-AudioCodec", "-AverageBitrate"]);
    let output = command.arg(path).output().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound { "ExifTool is not installed; install it to read embedded metadata.".into() }
        else { format!("Could not read embedded metadata: {error}") }
    })?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Could not read embedded metadata: {error}"))?;
    let Some(tags) = value.as_array().and_then(|items| items.first()).and_then(serde_json::Value::as_object) else {
        return Ok(Vec::new());
    };
    let mut sections: Vec<Section> = Vec::new();
    for (section, label, keys) in FIELDS {
        let Some((_, value)) = tags.iter().find(|(key, value)| {
            let Some(text) = value_text(value) else { return false; };
            !text.is_empty() && keys.iter().any(|alias| tag_name(key).eq_ignore_ascii_case(alias))
        }) else { continue; };
        let Some(value) = value_text(value) else { continue; };
        let value = clean_language_prefix(value);
        if let Some(group) = sections.iter_mut().find(|group| group.title == *section) {
            group.fields.push((label, value));
        } else {
            sections.push(Section { title: section, fields: vec![(label, value)] });
        }
    }
    Ok(sections)
}

fn tag_name(key: &str) -> &str {
    key.rsplit_once(':').map(|(_, name)| name).unwrap_or(key)
}

fn value_text(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.trim().to_owned()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Array(items) => {
            let values: Vec<_> = items.iter().filter_map(value_text).filter(|text| !text.is_empty()).collect();
            (!values.is_empty()).then(|| values.join(", "))
        }
        _ => None,
    }
}

fn clean_language_prefix(value: String) -> String {
    if let Some(rest) = value.strip_prefix("lang=\"") {
        if let Some((_, text)) = rest.split_once('"') { return text.trim().to_owned(); }
    }
    value
}
