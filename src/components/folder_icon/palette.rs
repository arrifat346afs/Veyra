//! Recolor SVG paint values while leaving paths, opacity, paper and highlights intact.
//! Papirus uses the brightest colored paint for the folder face, with darker
//! variants for its back and glyph. Their brightness ratios retain those layers.
use std::collections::BTreeMap;

fn paints(svg: &str) -> Vec<(std::ops::Range<usize>, [u8; 3])> {
    let bytes = svg.as_bytes();
    let mut result = Vec::new();
    for (index, _) in svg.match_indices('#') {
        let prefix = svg[..index].trim_end_matches([' ', '\t', '\n', '\r', '"', '\'']);
        let Some(prefix) = prefix
            .strip_suffix(':')
            .or_else(|| prefix.strip_suffix('='))
        else {
            continue;
        };
        let property = prefix
            .trim_end()
            .rsplit(|c: char| !c.is_ascii_alphabetic() && c != '-')
            .next()
            .unwrap_or("");
        if !["fill", "stroke", "color", "stop-color"].contains(&property) {
            continue;
        }
        let length = bytes[index + 1..]
            .iter()
            .take_while(|c| c.is_ascii_hexdigit())
            .count();
        if length != 3 && length != 6 && length != 8 {
            continue;
        }
        let hex = &svg[index + 1..index + 1 + length];
        let rgb = if length == 3 {
            let n = u16::from_str_radix(hex, 16).unwrap();
            [
                ((n >> 8) & 15) as u8 * 17,
                ((n >> 4) & 15) as u8 * 17,
                (n & 15) as u8 * 17,
            ]
        } else {
            let n = u32::from_str_radix(&hex[..6], 16).unwrap();
            [(n >> 16) as u8, (n >> 8) as u8, n as u8]
        };
        // Neutral whites and blacks are the paper, glints and cast shadows.
        let max = *rgb.iter().max().unwrap();
        let min = *rgb.iter().min().unwrap();
        if max - min < 16 && (min >= 210 || max <= 20) {
            continue;
        }
        result.push((index..index + 1 + length, rgb));
    }
    result
}
fn brightness(rgb: [u8; 3]) -> f32 {
    0.2126 * rgb[0] as f32 + 0.7152 * rgb[1] as f32 + 0.0722 * rgb[2] as f32
}
pub fn recolor(svg: &str, accent: [u8; 3]) -> String {
    let paints = paints(svg);
    let peak = paints
        .iter()
        .map(|(_, rgb)| brightness(*rgb))
        .fold(1., f32::max);
    let mut replacements = BTreeMap::new();
    for (range, rgb) in &paints {
        let shade = (brightness(*rgb) / peak).clamp(0., 1.);
        let [r, g, b] = accent.map(|c| (c as f32 * shade).round() as u8);
        let alpha = if range.len() == 9 {
            &svg[range.end - 2..range.end]
        } else {
            ""
        };
        replacements.insert(
            range.start,
            (range.end, format!("#{r:02x}{g:02x}{b:02x}{alpha}")),
        );
    }
    let mut output = String::with_capacity(svg.len());
    let mut cursor = 0;
    for (start, (end, color)) in replacements {
        output.push_str(&svg[cursor..start]);
        output.push_str(&color);
        cursor = end;
    }
    output.push_str(&svg[cursor..]);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    const SVG: &str = r##"<svg><rect fill="#e4e4e4"/><path style="fill:#4877b1" d="M 4,46"/><rect fill="#5294e2"/><path fill="#1d344f"/><path opacity="0.1" fill="#ffffff"/><path fill="#000000"/></svg>"##;
    #[test]
    fn palette_changes_keep_geometry_and_neutral_details() {
        let peach = recolor(SVG, [255, 180, 165]);
        assert!(peach.contains("#ffb4a5"));
        for detail in [
            "#e4e4e4",
            "#ffffff",
            "#000000",
            "d=\"M 4,46\"",
            "opacity=\"0.1\"",
        ] {
            assert!(peach.contains(detail));
        }
        assert!(!peach.contains("#5294e2"));
        assert!(!peach.contains("#4877b1"));
        assert!(!peach.contains("#1d344f"));
        assert_ne!(peach, recolor(SVG, [100, 180, 255]));
        let paints = paints(&peach);
        assert!(brightness(paints[2].1) < brightness(paints[1].1));
    }
    #[test]
    fn only_paint_values_change_and_short_hex_and_alpha_work() {
        let svg = r##"<svg id="abcdef"><path fill="url(#abcdef)"/><rect fill="#39c8"/><rect style="fill: #39c"/><rect fill="#3399cc80"/></svg>"##;
        let result = recolor(svg, [255, 0, 0]);
        assert!(result.contains("url(#abcdef)"));
        assert!(result.contains("#ff000080"));
        assert!(result.contains("fill: #ff0000"));
        // Unsupported paint formats remain untouched rather than corrupting SVG.
        assert!(result.contains("#39c8"));
    }
}
