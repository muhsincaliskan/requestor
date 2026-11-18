use eframe::egui::{
    vec2, Button, Color32, Painter, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};

pub const ICON_BUTTON_SIZE: f32 = 22.0;
const ICON_STROKE_WIDTH: f32 = 1.3;
const HOVER_INSET: f32 = 3.0;
const CLOSE_CROSS_SIZE: f32 = 7.0;
const CHEVRON_HALF_WIDTH: f32 = 4.0;
const CHEVRON_HALF_HEIGHT: f32 = 2.0;
const EYE_HALF_WIDTH: f32 = 6.0;
const EYE_HALF_HEIGHT: f32 = 3.5;
const EYE_PUPIL_RADIUS: f32 = 1.6;
const EYE_SEGMENTS: usize = 12;
const SHOW_HINT: &str = "Show value";
const HIDE_HINT: &str = "Hide value";

pub fn icon_button(ui: &mut Ui, glyph: &str, hint: &str) -> Response {
    ui.add(
        Button::new(glyph)
            .frame(false)
            .min_size(vec2(ICON_BUTTON_SIZE, ICON_BUTTON_SIZE)),
    )
    .on_hover_text(hint)
}

pub fn close_button(ui: &mut Ui, hint: &str) -> Response {
    painted_button(ui, hint, |painter, rect, stroke| {
        let cross = Rect::from_center_size(rect.center(), Vec2::splat(CLOSE_CROSS_SIZE));
        painter.line_segment([cross.left_top(), cross.right_bottom()], stroke);
        painter.line_segment([cross.right_top(), cross.left_bottom()], stroke);
    })
}

pub fn chevron_button(ui: &mut Ui, hint: &str) -> Response {
    painted_button(ui, hint, |painter, rect, stroke| {
        let center = rect.center();
        let points = vec![
            center + vec2(-CHEVRON_HALF_WIDTH, -CHEVRON_HALF_HEIGHT),
            center + vec2(0.0, CHEVRON_HALF_HEIGHT),
            center + vec2(CHEVRON_HALF_WIDTH, -CHEVRON_HALF_HEIGHT),
        ];
        painter.add(Shape::line(points, stroke));
    })
}

pub fn eye_button(ui: &mut Ui, revealed: bool) -> Response {
    let hint = if revealed { HIDE_HINT } else { SHOW_HINT };
    painted_button(ui, hint, |painter, rect, stroke| {
        let center = rect.center();
        let outline: Vec<_> = (0..=EYE_SEGMENTS * 2)
            .map(|step| {
                let angle = std::f32::consts::PI * step as f32 / EYE_SEGMENTS as f32;
                center + vec2(EYE_HALF_WIDTH * angle.cos(), EYE_HALF_HEIGHT * angle.sin())
            })
            .collect();
        painter.add(Shape::closed_line(outline, stroke));
        painter.circle_filled(center, EYE_PUPIL_RADIUS, stroke.color);
        if revealed {
            let corner = vec2(EYE_HALF_WIDTH, EYE_HALF_WIDTH);
            painter.line_segment([center - corner, center + corner], stroke);
        }
    })
}

fn painted_button(ui: &mut Ui, hint: &str, paint: impl FnOnce(&Painter, Rect, Stroke)) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(ICON_BUTTON_SIZE), Sense::click());
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        if response.hovered() {
            ui.painter().rect_filled(
                rect.shrink(HOVER_INSET),
                visuals.corner_radius,
                visuals.bg_fill,
            );
        }
        let color: Color32 = visuals.fg_stroke.color;
        paint(ui.painter(), rect, Stroke::new(ICON_STROKE_WIDTH, color));
    }
    response.on_hover_text(hint)
}
