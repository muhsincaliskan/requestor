use eframe::egui::{Color32, Sense, Stroke, Ui, Vec2};
use requestor_core::workspace::{AccentName, InterfaceSettings};

use crate::theme::Palette;

const TITLE: &str = "Accent color";
const INTRO: &str = "Used for links, selections and the main buttons.";
const DOT_DIAMETER: f32 = 22.0;
const DOT_SPACING: f32 = 10.0;
const RING_GAP: f32 = 3.0;
const RING_WIDTH: f32 = 2.0;
const THEME_RING_WIDTH: f32 = 5.0;
const SECTION_GAP: f32 = 16.0;

pub fn show(ui: &mut Ui, settings: &mut InterfaceSettings) -> bool {
    ui.add_space(SECTION_GAP);
    ui.strong(TITLE);
    ui.weak(INTRO);
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(DOT_SPACING);
        for accent in AccentName::ALL {
            let color = shown_color(ui, settings, accent);
            if dot(ui, accent, color, settings.accent == accent) && settings.accent != accent {
                settings.accent = accent;
                changed = true;
            }
        }
    });
    changed
}

fn shown_color(ui: &Ui, settings: &InterfaceSettings, accent: AccentName) -> Color32 {
    Palette::of(settings.theme, accent, ui.visuals().dark_mode).accent
}

fn dot(ui: &mut Ui, accent: AccentName, color: Color32, selected: bool) -> bool {
    let size = Vec2::splat(DOT_DIAMETER + 2.0 * (RING_GAP + RING_WIDTH));
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter();
    if accent == AccentName::Theme {
        let radius = (DOT_DIAMETER - THEME_RING_WIDTH) / 2.0;
        painter.circle_stroke(rect.center(), radius, Stroke::new(THEME_RING_WIDTH, color));
    } else {
        painter.circle_filled(rect.center(), DOT_DIAMETER / 2.0, color);
    }
    if selected || response.hovered() {
        let ring = if selected {
            Stroke::new(RING_WIDTH, color)
        } else {
            ui.visuals().widgets.hovered.bg_stroke
        };
        painter.circle_stroke(rect.center(), DOT_DIAMETER / 2.0 + RING_GAP, ring);
    }
    response.on_hover_text(accent.label()).clicked()
}
