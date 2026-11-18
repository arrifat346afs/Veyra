//! User-selected geometry overrides theme files, including live theme reloads.
use gpui_kit::{App, px};
use gpui_kit::component::Theme;

pub fn configure(theme: &mut Theme, value: usize) {
    let radius = px(value.min(24) as f32);
    theme.radius = radius;
    theme.radius_lg = radius;
    theme.window_radius = radius;
}

pub fn apply(value: usize, cx: &mut App) {
    Theme::update(cx, |theme| configure(theme, value));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::preferences::Preferences;

    #[test]
    fn old_preferences_get_default_radius_and_new_value_round_trips() {
        let mut preferences: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(preferences.corner_radius, 8);
        preferences.corner_radius = 17;
        let saved = serde_json::to_string(&preferences).unwrap();
        assert_eq!(serde_json::from_str::<Preferences>(&saved).unwrap().corner_radius, 17);
    }

    #[test]
    fn user_radius_overrides_theme_geometry_and_clamps_to_slider_range() {
        let mut theme = Theme::default();
        for (value, expected) in [(0, 0.), (17, 17.), (usize::MAX, 24.)] {
            configure(&mut theme, value);
            assert_eq!(theme.radius, px(expected));
            assert_eq!(theme.radius_lg, px(expected));
            assert_eq!(theme.window_radius, px(expected));
            assert_eq!(theme.semantic_tokens().radius.md, px(expected));
        }
    }
}
