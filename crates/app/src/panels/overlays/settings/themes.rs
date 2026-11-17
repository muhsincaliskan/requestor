use eframe::egui::{
    Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2,
};
use requestor_core::workspace::{InterfaceSettings, ThemeName};

const CARD_SIZE: Vec2 = Vec2::new(150.0, 86.0);
const SWATCH_HEIGHT: f32 = 52.0;
const CARD_RADIUS: u8 = 6;
const CARD_SPACING: f32 = 10.0;
const LABEL_BASELINE: f32 = 14.0;
const SELECTED_STROKE_WIDTH: f32 = 2.0;
const PANEL_SHARE: f32 = 0.35;
const ACCENT_OFFSET: Vec2 = Vec2::new(0.45, 0.2);
const ACCENT_SIZE: Vec2 = Vec2::new(0.4, 0.2);
const ACCENT_RADIUS: u8 = 2;

struct Swatch {
    background: Color32,
    panel: Color32,
    accent: Color32,
}

const LIGHT: Swatch = Swatch {
    background: Color32::from_rgb(255, 255, 255),
    panel: Color32::from_rgb(238, 240, 243),
    accent: Color32::from_rgb(0, 109, 255),
};
const DARK: Swatch = Swatch {
    background: Color32::from_rgb(27, 27, 27),
    panel: Color32::from_rgb(44, 44, 44),
    accent: Color32::from_rgb(0, 109, 255),
};
const MIDNIGHT: Swatch = Swatch {
    background: Color32::from_rgb(18, 23, 38),
    panel: Color32::from_rgb(22, 28, 46),
    accent: Color32::from_rgb(64, 110, 214),
};
const SEPIA: Swatch = Swatch {
    background: Color32::from_rgb(249, 243, 228),
    panel: Color32::from_rgb(236, 226, 201),
    accent: Color32::from_rgb(196, 148, 86),
};

fn swatches(theme: ThemeName) -> [&'static Swatch; 2] {
    match theme {
        ThemeName::System => [&LIGHT, &DARK],
        ThemeName::Light => [&LIGHT, &LIGHT],
        ThemeName::Dark => [&DARK, &DARK],
        ThemeName::Midnight => [&MIDNIGHT, &MIDNIGHT],
        ThemeName::Sepia => [&SEPIA, &SEPIA],
    }
}

pub fn show(ui: &mut Ui, settings: &mut InterfaceSettings) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(CARD_SPACING);
        for theme in ThemeName::ALL {
            if card(ui, theme, settings.theme == theme) && settings.theme != theme {
                settings.theme = theme;
                changed = true;
            }
        }
    });
    changed
}

fn card(ui: &mut Ui, theme: ThemeName, selected: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(CARD_SIZE, Sense::click());
    let radius = CornerRadius::same(CARD_RADIUS);
    let visuals = ui.visuals();
    let painter = ui.painter();
    painter.rect_filled(rect, radius, visuals.faint_bg_color);
    paint_swatch(ui, rect, theme);
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

fn paint_swatch(ui: &Ui, card: Rect, theme: ThemeName) {
    let area = Rect::from_min_size(card.min, Vec2::new(card.width(), SWATCH_HEIGHT));
    let halves = swatches(theme);
    let half_width = area.width() / halves.len() as f32;
    for (index, swatch) in halves.into_iter().enumerate() {
        let origin = area.min + Vec2::new(half_width * index as f32, 0.0);
        let half = Rect::from_min_size(origin, Vec2::new(half_width, area.height()));
        paint_half(ui, half, swatch);
    }
}

fn paint_half(ui: &Ui, half: Rect, swatch: &Swatch) {
    let panel = Rect::from_min_size(
        half.min,
        Vec2::new(half.width() * PANEL_SHARE, half.height()),
    );
    let accent = Rect::from_min_size(
        half.min + half.size() * ACCENT_OFFSET,
        half.size() * ACCENT_SIZE,
    );
    let painter = ui.painter();
    painter.rect_filled(half, CornerRadius::ZERO, swatch.background);
    painter.rect_filled(panel, CornerRadius::ZERO, swatch.panel);
    painter.rect_filled(accent, CornerRadius::same(ACCENT_RADIUS), swatch.accent);
}
