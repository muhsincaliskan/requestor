use serde::{Deserialize, Serialize};

use crate::settings::RequestSettings;

const DEFAULT_INTERFACE_SCALE: f32 = 1.0;
const MIN_INTERFACE_SCALE: f32 = 0.75;
const MAX_INTERFACE_SCALE: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeName {
    Light,
    Dark,
    Midnight,
    Sepia,
    #[default]
    #[serde(other)]
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceSettings {
    pub theme: ThemeName,
    pub interface_scale: f32,
    pub autosave: bool,
    pub variable_autocomplete: bool,
    pub response_format_detection: bool,
    pub retain_headers_on_links: bool,
    pub open_sidebar_item_in_new_tab: bool,
    pub ask_when_closing_unsaved_tabs: bool,
    pub show_icons_with_tab_names: bool,
    pub two_pane_view: bool,
    pub show_workbench: bool,
    pub show_sidebar: bool,
    pub sidebars_swapped: bool,
    pub check_for_updates: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub request: RequestSettings,
    pub interface: InterfaceSettings,
}

impl ThemeName {
    pub const ALL: [ThemeName; 5] = [
        ThemeName::System,
        ThemeName::Light,
        ThemeName::Dark,
        ThemeName::Midnight,
        ThemeName::Sepia,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeName::System => "System",
            ThemeName::Light => "Light",
            ThemeName::Dark => "Dark",
            ThemeName::Midnight => "Midnight",
            ThemeName::Sepia => "Sepia",
        }
    }
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        InterfaceSettings {
            theme: ThemeName::System,
            interface_scale: DEFAULT_INTERFACE_SCALE,
            autosave: true,
            variable_autocomplete: true,
            response_format_detection: true,
            retain_headers_on_links: false,
            open_sidebar_item_in_new_tab: false,
            ask_when_closing_unsaved_tabs: true,
            show_icons_with_tab_names: true,
            two_pane_view: false,
            show_workbench: true,
            show_sidebar: true,
            sidebars_swapped: false,
            check_for_updates: true,
        }
    }
}

impl InterfaceSettings {
    pub const SCALE_RANGE: std::ops::RangeInclusive<f32> =
        MIN_INTERFACE_SCALE..=MAX_INTERFACE_SCALE;

    pub fn effective_scale(&self) -> f32 {
        if self.interface_scale.is_finite() {
            self.interface_scale
                .clamp(MIN_INTERFACE_SCALE, MAX_INTERFACE_SCALE)
        } else {
            DEFAULT_INTERFACE_SCALE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, InterfaceSettings, ThemeName};

    #[test]
    fn autosave_and_autocomplete_are_on_by_default() {
        let settings = AppSettings::default();
        assert!(settings.interface.autosave);
        assert!(settings.interface.variable_autocomplete);
        assert!(settings.interface.ask_when_closing_unsaved_tabs);
        assert!(settings.request.verify_ssl);
    }

    #[test]
    fn partial_and_unknown_values_fall_back_to_defaults() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"interface":{"theme":"neon","two_pane_view":true,"interface_scale":9.0}}"#,
        )
        .unwrap();
        assert_eq!(settings.interface.theme, ThemeName::System);
        assert!(settings.interface.two_pane_view);
        assert!(settings.interface.autosave);
        assert_eq!(settings.interface.effective_scale(), 2.0);
    }

    #[test]
    fn theme_names_are_stored_in_snake_case() {
        let interface = InterfaceSettings {
            theme: ThemeName::Midnight,
            ..InterfaceSettings::default()
        };
        assert!(serde_json::to_string(&interface)
            .unwrap()
            .contains(r#""theme":"midnight""#));
    }
}
