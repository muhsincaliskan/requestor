use eframe::egui::{Context, Theme, ThemePreference, Visuals};
use requestor_core::workspace::{AccentName, InterfaceSettings, ThemeName};

mod accent;
mod contrast;
mod palette;
mod style;
mod typeface;

pub use contrast::label_on;
pub use typeface::install as install_fonts;

pub use palette::Palette;

#[derive(Clone, Copy, PartialEq)]
pub struct Look {
    theme: ThemeName,
    accent: AccentName,
    scale: f32,
}

impl Look {
    pub fn of(interface: &InterfaceSettings) -> Self {
        Look {
            theme: interface.theme,
            accent: interface.accent,
            scale: interface.effective_scale(),
        }
    }

    pub fn apply(self, ctx: &Context) {
        style::apply(ctx);
        ctx.set_visuals_of(Theme::Dark, self.dark_visuals());
        ctx.set_visuals_of(Theme::Light, self.light_visuals());
        ctx.set_theme(self.preference());
        ctx.set_zoom_factor(self.scale);
    }

    fn preference(self) -> ThemePreference {
        match self.theme {
            ThemeName::System => ThemePreference::System,
            ThemeName::Light | ThemeName::Sepia => ThemePreference::Light,
            ThemeName::Dark | ThemeName::Midnight => ThemePreference::Dark,
        }
    }

    fn dark_visuals(self) -> Visuals {
        Palette::of(self.theme, self.accent, true).tint(Visuals::dark())
    }

    fn light_visuals(self) -> Visuals {
        Palette::of(self.theme, self.accent, false).tint(Visuals::light())
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::Color32;
    use requestor_core::workspace::{AccentName, InterfaceSettings, ThemeName};

    use super::palette::{DAYLIGHT, NIGHT};
    use super::Look;

    fn look(accent: AccentName) -> Look {
        Look::of(&InterfaceSettings {
            theme: ThemeName::System,
            accent,
            ..InterfaceSettings::default()
        })
    }

    #[test]
    fn a_chosen_accent_is_used_in_both_modes() {
        let purple = look(AccentName::Purple);
        assert_eq!(
            purple.light_visuals().hyperlink_color,
            Color32::from_rgb(118, 62, 200)
        );
        assert_eq!(
            purple.dark_visuals().hyperlink_color,
            Color32::from_rgb(170, 140, 250)
        );
    }

    #[test]
    fn the_theme_accent_keeps_the_palette_colour() {
        let theme = look(AccentName::Theme);
        assert_eq!(theme.light_visuals().hyperlink_color, DAYLIGHT.accent);
        assert_eq!(theme.dark_visuals().hyperlink_color, NIGHT.accent);
    }
}
