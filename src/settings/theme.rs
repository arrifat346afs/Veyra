use gpui_kit::component::{Theme, ThemeConfig, ThemeSet};
use std::{fs, path::Path, rc::Rc};

/// Load either a GPUI Kit theme file or a flat Matugen/Noctalia palette.
pub fn load(path: &Path) -> Result<ThemeConfig, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("Could not read theme file: {e}"))?;
    parse(&source)
}

pub(super) fn parse(source: &str) -> Result<ThemeConfig, String> {
    let value: serde_json::Value =
        serde_json::from_str(source).map_err(|e| format!("Invalid theme JSON: {e}"))?;
    if value.get("themes").is_some() {
        let set: ThemeSet =
            serde_json::from_value(value).map_err(|e| format!("Invalid theme file: {e}"))?;
        return set
            .themes
            .into_iter()
            .next()
            .ok_or_else(|| "Theme file contains no themes".into());
    }
    let object = value.as_object().ok_or("Theme must be a JSON object")?;
    let color = |key: &str, fallback: &str| -> Result<String, String> {
        let text = object
            .get(key)
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| format!("{key} must be a color string"))
            })
            .transpose()?
            .unwrap_or(fallback);
        if text.len() != 7 || !text.starts_with('#') || u32::from_str_radix(&text[1..], 16).is_err()
        {
            return Err(format!("{key} must be a #RRGGBB color"));
        }
        Ok(text.to_owned())
    };
    let background = color("background", "#200f0c")?;
    let foreground = color("backgroundText", "#fddbd4")?;
    let surface = color("surface", &background)?;
    let surface_text = color("surfaceText", &foreground)?;
    let muted = color("surfaceContainerLow", &surface)?;
    let hover = color("surfaceContainerHigh", &muted)?;
    let active = color("surfaceContainerHighest", &hover)?;
    let muted_text = color("surfaceVariantText", &foreground)?;
    let primary = color("primary", &foreground)?;
    let primary_text = color("primaryText", &background)?;
    let secondary = color("secondary", &primary)?;
    let border = color("outline", &muted_text)?;
    let error = color("error", &primary)?;
    let warning = color("warning", &primary)?;
    let info = color("info", &primary)?;
    let mut colors = serde_json::Map::new();
    // Supply every independent palette root. Component state colors then derive
    // from these roots rather than the built-in neutral and blue palette.
    for (keys, value) in [
        (
            vec![
                "background",
                "list.background",
                "table.background",
                "scrollbar.background",
                "overlay",
            ],
            &background,
        ),
        (
            vec![
                "foreground",
                "popover.foreground",
                "sidebar.foreground",
                "group_box.foreground",
            ],
            &foreground,
        ),
        (
            vec![
                "popover.background",
                "accordion.background",
                "tab.background",
                "tab.active.background",
                "tab_bar.background",
            ],
            &surface,
        ),
        (
            vec![
                "muted.background",
                "sidebar.background",
                "group_box.background",
                "button.background",
                "switch.background",
                "secondary.background",
            ],
            &muted,
        ),
        (
            vec!["muted.foreground", "description_list.label.foreground"],
            &muted_text,
        ),
        (
            vec![
                "accent.background",
                "list.hover.background",
                "button.hover.background",
                "secondary.hover.background",
            ],
            &hover,
        ),
        (
            vec![
                "accent.foreground",
                "button.foreground",
                "secondary.foreground",
                "sidebar.accent.foreground",
            ],
            &surface_text,
        ),
        (
            vec!["button.active.background", "secondary.active.background"],
            &active,
        ),
        (
            vec![
                "primary.background",
                "ring",
                "caret",
                "link",
                "link.active",
                "link.hover",
                "selection.background",
                "list.active.border",
                "drag.border",
                "slider.background",
                "scrollbar.thumb.background",
            ],
            &primary,
        ),
        (
            vec!["primary.foreground", "slider.thumb.background"],
            &primary_text,
        ),
        (
            vec![
                "switch.thumb.background",
                "base.cyan",
                "base.magenta",
                "base.green",
            ],
            &secondary,
        ),
        (
            vec!["border", "input.border", "sidebar.border", "window.border"],
            &border,
        ),
        (vec!["base.blue"], &primary),
        (vec!["danger.background", "base.red"], &error),
        (vec!["warning.background", "base.yellow"], &warning),
        (vec!["info.background"], &info),
    ] {
        for key in keys {
            colors.insert(key.into(), value.clone().into());
        }
    }
    let is_light = value
        .get("background")
        .and_then(|v| v.as_str())
        .map(is_light_color)
        .unwrap_or(false);
    let config = serde_json::json!({
        "name": value.get("name").and_then(|v| v.as_str()).unwrap_or("Custom"),
        "mode": if is_light {"light"} else {"dark"},
        "colors": colors,
    });
    serde_json::from_value(config).map_err(|e| format!("Invalid palette colors: {e}"))
}

fn is_light_color(hex: &str) -> bool {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return false;
    }
    u32::from_str_radix(hex, 16)
        .ok()
        .map(|rgb| {
            let r = (rgb >> 16) & 255;
            let g = (rgb >> 8) & 255;
            let b = rgb & 255;
            (299 * r + 587 * g + 114 * b) / 1000 > 128
        })
        .unwrap_or(false)
}

pub fn apply(path: &str, cx: &mut gpui_kit::App) -> Result<(), String> {
    if path.trim().is_empty() {
        return Ok(());
    }
    let config = Rc::new(load(Path::new(path.trim()))?);
    Theme::update(cx, |theme| theme.apply_config(&config));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Hsla, rgb};

    #[test]
    fn flat_palette_replaces_default_selection_focus_and_controls() {
        let config = Rc::new(
            parse(
                r##"{
            "background":"#200f0c", "backgroundText":"#fddbd4",
            "primary":"#ffb4a5", "primaryText":"#650900",
            "outline":"#ae8881", "surfaceContainerLow":"#291713"
        }"##,
            )
            .unwrap(),
        );
        let mut theme = Theme::default();
        theme.apply_config(&config);
        let primary: Hsla = rgb(0xffb4a5).into();
        let foreground: Hsla = rgb(0x650900).into();
        assert_eq!(theme.primary, primary);
        assert_eq!(theme.primary_foreground, foreground);
        assert_eq!(theme.button_primary, primary);
        assert_eq!(theme.ring, primary);
        assert_eq!(theme.caret, primary);
        assert_eq!(theme.slider_bar, primary);
        assert_eq!(theme.switch, rgb(0x291713).into());
        assert_eq!(theme.border, rgb(0xae8881).into());
        assert_eq!(theme.selection.h, primary.h);
        assert_eq!(theme.list_active_border, primary);
    }

    #[test]
    fn malformed_palette_color_is_reported() {
        assert!(parse(r##"{"primary":"not-a-color"}"##).is_err());
    }
}
