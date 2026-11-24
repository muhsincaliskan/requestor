use eframe::egui::{Color32, CornerRadius, Stroke, Visuals};
use requestor_core::workspace::{AccentName, ThemeName};

use super::accent::accent_color;

const WIDGET_RADIUS: u8 = 10;
const WINDOW_RADIUS: u8 = 14;
const MENU_RADIUS: u8 = 12;
const BORDER_WIDTH: f32 = 1.0;
const SELECTION_STRENGTH: f32 = 0.3;

#[derive(Clone, Copy)]
pub struct Palette {
    pub panel: Color32,
    pub window: Color32,
    pub input: Color32,
    pub stripe: Color32,
    pub text: Color32,
    pub widget: Color32,
    pub widget_hover: Color32,
    pub widget_active: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub danger: Color32,
    pub muted: Color32,
}

pub const DAYLIGHT: Palette = Palette {
    panel: Color32::from_rgb(245, 246, 248),
    window: Color32::from_rgb(255, 255, 255),
    input: Color32::from_rgb(255, 255, 255),
    stripe: Color32::from_rgb(238, 240, 244),
    text: Color32::from_rgb(31, 35, 41),
    widget: Color32::from_rgb(234, 237, 242),
    widget_hover: Color32::from_rgb(222, 227, 236),
    widget_active: Color32::from_rgb(208, 215, 228),
    border: Color32::from_rgb(214, 219, 227),
    accent: Color32::from_rgb(37, 99, 235),
    danger: Color32::from_rgb(208, 32, 32),
    muted: Color32::from_rgb(96, 103, 115),
};

pub const NIGHT: Palette = Palette {
    panel: Color32::from_rgb(24, 26, 31),
    window: Color32::from_rgb(31, 34, 41),
    input: Color32::from_rgb(16, 18, 22),
    stripe: Color32::from_rgb(33, 36, 43),
    text: Color32::from_rgb(225, 229, 236),
    widget: Color32::from_rgb(44, 48, 57),
    widget_hover: Color32::from_rgb(55, 60, 71),
    widget_active: Color32::from_rgb(66, 72, 85),
    border: Color32::from_rgb(54, 59, 70),
    accent: Color32::from_rgb(76, 139, 255),
    danger: Color32::from_rgb(239, 83, 80),
    muted: Color32::from_rgb(145, 148, 154),
};

pub const MIDNIGHT: Palette = Palette {
    panel: Color32::from_rgb(13, 17, 28),
    window: Color32::from_rgb(18, 23, 38),
    input: Color32::from_rgb(8, 11, 19),
    stripe: Color32::from_rgb(22, 28, 46),
    text: Color32::from_rgb(201, 209, 229),
    widget: Color32::from_rgb(30, 38, 62),
    widget_hover: Color32::from_rgb(40, 50, 80),
    widget_active: Color32::from_rgb(50, 62, 98),
    border: Color32::from_rgb(38, 47, 74),
    accent: Color32::from_rgb(64, 110, 214),
    danger: Color32::from_rgb(224, 82, 82),
    muted: Color32::from_rgb(126, 132, 149),
};

pub const SEPIA: Palette = Palette {
    panel: Color32::from_rgb(244, 236, 216),
    window: Color32::from_rgb(249, 243, 228),
    input: Color32::from_rgb(253, 249, 238),
    stripe: Color32::from_rgb(236, 226, 201),
    text: Color32::from_rgb(73, 56, 38),
    widget: Color32::from_rgb(232, 221, 194),
    widget_hover: Color32::from_rgb(222, 209, 178),
    widget_active: Color32::from_rgb(210, 195, 160),
    border: Color32::from_rgb(214, 201, 170),
    accent: Color32::from_rgb(176, 120, 52),
    danger: Color32::from_rgb(178, 58, 42),
    muted: Color32::from_rgb(112, 92, 68),
};

impl Palette {
    pub fn of(theme: ThemeName, accent: AccentName, dark_mode: bool) -> Palette {
        let base = match (theme, dark_mode) {
            (ThemeName::Midnight, true) => MIDNIGHT,
            (ThemeName::Sepia, false) => SEPIA,
            (_, true) => NIGHT,
            (_, false) => DAYLIGHT,
        };
        Palette {
            accent: accent_color(accent, dark_mode).unwrap_or(base.accent),
            ..base
        }
    }

    pub fn tint(&self, mut visuals: Visuals) -> Visuals {
        visuals.panel_fill = self.panel;
        visuals.window_fill = self.window;
        visuals.extreme_bg_color = self.input;
        visuals.faint_bg_color = self.stripe;
        visuals.override_text_color = Some(self.text);
        visuals.selection.bg_fill = self.panel.lerp_to_gamma(self.accent, SELECTION_STRENGTH);
        visuals.selection.stroke = Stroke::new(BORDER_WIDTH, self.accent);
        visuals.hyperlink_color = self.accent;
        visuals.error_fg_color = self.danger;
        visuals.weak_text_color = Some(self.muted);
        visuals.window_stroke = Stroke::new(BORDER_WIDTH, self.border);
        visuals.window_corner_radius = CornerRadius::same(WINDOW_RADIUS);
        visuals.menu_corner_radius = CornerRadius::same(MENU_RADIUS);
        self.tint_widgets(&mut visuals);
        visuals
    }

    fn tint_widgets(&self, visuals: &mut Visuals) {
        let radius = CornerRadius::same(WIDGET_RADIUS);
        let border = Stroke::new(BORDER_WIDTH, self.border);
        let states = [
            (&mut visuals.widgets.inactive, self.widget),
            (&mut visuals.widgets.hovered, self.widget_hover),
            (&mut visuals.widgets.active, self.widget_active),
            (&mut visuals.widgets.open, self.widget_hover),
        ];
        for (state, fill) in states {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = border;
            state.corner_radius = radius;
            state.fg_stroke.color = self.text;
        }
        visuals.widgets.noninteractive.bg_stroke = border;
        visuals.widgets.noninteractive.corner_radius = radius;
    }
}
