use eframe::egui::{Context, Theme, ThemePreference, Visuals};
use requestor_core::workspace::{InterfaceSettings, ThemeName};

mod contrast;
mod palette;
mod style;

pub use contrast::label_on;

use palette::{DAYLIGHT, MIDNIGHT, NIGHT, SEPIA};

#[derive(Clone, Copy, PartialEq)]
pub struct Look {
    theme: ThemeName,
    scale: f32,
}

impl Look {
    pub fn of(interface: &InterfaceSettings) -> Self {
        Look {
            theme: interface.theme,
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
        match self.theme {
            ThemeName::Midnight => MIDNIGHT.tint(Visuals::dark()),
            _ => NIGHT.tint(Visuals::dark()),
        }
    }

    fn light_visuals(self) -> Visuals {
        match self.theme {
            ThemeName::Sepia => SEPIA.tint(Visuals::light()),
            _ => DAYLIGHT.tint(Visuals::light()),
        }
    }
}
