use eframe::egui::{Color32, Context, Theme, ThemePreference, Visuals};
use requestor_core::workspace::{InterfaceSettings, ThemeName};

struct Palette {
    panel: Color32,
    window: Color32,
    input: Color32,
    stripe: Color32,
    text: Color32,
    accent: Color32,
}

const MIDNIGHT: Palette = Palette {
    panel: Color32::from_rgb(13, 17, 28),
    window: Color32::from_rgb(18, 23, 38),
    input: Color32::from_rgb(8, 11, 19),
    stripe: Color32::from_rgb(22, 28, 46),
    text: Color32::from_rgb(201, 209, 229),
    accent: Color32::from_rgb(64, 110, 214),
};

const SEPIA: Palette = Palette {
    panel: Color32::from_rgb(244, 236, 216),
    window: Color32::from_rgb(249, 243, 228),
    input: Color32::from_rgb(253, 249, 238),
    stripe: Color32::from_rgb(236, 226, 201),
    text: Color32::from_rgb(73, 56, 38),
    accent: Color32::from_rgb(196, 148, 86),
};

impl Palette {
    fn tint(&self, mut visuals: Visuals) -> Visuals {
        visuals.panel_fill = self.panel;
        visuals.window_fill = self.window;
        visuals.extreme_bg_color = self.input;
        visuals.faint_bg_color = self.stripe;
        visuals.override_text_color = Some(self.text);
        visuals.selection.bg_fill = self.accent;
        visuals.hyperlink_color = self.accent;
        visuals
    }
}

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
            _ => Visuals::dark(),
        }
    }

    fn light_visuals(self) -> Visuals {
        match self.theme {
            ThemeName::Sepia => SEPIA.tint(Visuals::light()),
            _ => Visuals::light(),
        }
    }
}
