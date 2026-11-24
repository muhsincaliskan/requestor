use eframe::egui::{Align2, CornerRadius, Rect, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2};
use requestor_core::workspace::{AccentName, InterfaceSettings, ThemeName};

use super::accents;
use crate::theme::Palette;

const CARD_SIZE: Vec2 = Vec2::new(150.0, 86.0);
const SWATCH_HEIGHT: f32 = 52.0;
const CARD_RADIUS: u8 = 10;
const CARD_SPACING: f32 = 10.0;
const LABEL_BASELINE: f32 = 14.0;
const SELECTED_STROKE_WIDTH: f32 = 2.0;
const PANEL_SHARE: f32 = 0.35;
const ACCENT_OFFSET: Vec2 = Vec2::new(0.45, 0.2);
const ACCENT_SIZE: Vec2 = Vec2::new(0.4, 0.2);
const ACCENT_RADIUS: u8 = 2;

fn swatches(theme: ThemeName, accent: AccentName) -> [Palette; 2] {
    let modes = match theme {
        ThemeName::System => [false, true],
        ThemeName::Light | ThemeName::Sepia => [false, false],
        ThemeName::Dark | ThemeName::Midnight => [true, true],
    };
    modes.map(|dark_mode| Palette::of(theme, accent, dark_mode))
}

pub fn show(ui: &mut Ui, settings: &mut InterfaceSettings) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(CARD_SPACING);
        for theme in ThemeName::ALL {
            let selected = settings.theme == theme;
            if card(ui, (theme, settings.accent), selected) && !selected {
                settings.theme = theme;
                changed = true;
            }
        }
    });
    changed |= accents::show(ui, settings);
    changed
}

fn card(ui: &mut Ui, (theme, accent): (ThemeName, AccentName), selected: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(CARD_SIZE, Sense::click());
    let radius = CornerRadius::same(CARD_RADIUS);
    let visuals = ui.visuals();
    let painter = ui.painter();
    painter.rect_filled(rect, radius, visuals.faint_bg_color);
    paint_swatch(ui, rect, swatches(theme, accent));
    let stroke = if selected {
        Stroke::new(SELECTED_STROKE_WIDTH, visuals.selection.bg_fill)
    } else {
        visuals.widgets.noninteractive.bg_stroke
    };
    painter.rect_stroke(rect, radius, stroke, StrokeKind::Inside);
    painter.text(
        rect.left_bottom() + Vec2::new(CARD_SPACING, -LABEL_BASELINE),
        Align2::LEFT_CENTER,
        theme.label(),
        TextStyle::Body.resolve(ui.style()),
        visuals.text_color(),
    );
    response.clicked()
}

fn paint_swatch(ui: &Ui, card: Rect, halves: [Palette; 2]) {
    let area = Rect::from_min_size(card.min, Vec2::new(card.width(), SWATCH_HEIGHT));
    let half_width = area.width() / halves.len() as f32;
    for (index, swatch) in halves.into_iter().enumerate() {
        let origin = area.min + Vec2::new(half_width * index as f32, 0.0);
        let half = Rect::from_min_size(origin, Vec2::new(half_width, area.height()));
        paint_half(ui, half, &swatch);
    }
}

fn paint_half(ui: &Ui, half: Rect, swatch: &Palette) {
    let panel = Rect::from_min_size(
        half.min,
        Vec2::new(half.width() * PANEL_SHARE, half.height()),
    );
    let accent = Rect::from_min_size(
        half.min + half.size() * ACCENT_OFFSET,
        half.size() * ACCENT_SIZE,
    );
    let painter = ui.painter();
    painter.rect_filled(half, CornerRadius::ZERO, swatch.window);
    painter.rect_filled(panel, CornerRadius::ZERO, swatch.panel);
    painter.rect_filled(accent, CornerRadius::same(ACCENT_RADIUS), swatch.accent);
}
