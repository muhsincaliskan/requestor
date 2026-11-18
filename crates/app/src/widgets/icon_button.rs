use eframe::egui::{
    vec2, Button, Color32, Painter, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};

pub const ICON_BUTTON_SIZE: f32 = 22.0;
const ICON_STROKE_WIDTH: f32 = 1.3;
const HOVER_INSET: f32 = 3.0;
const CLOSE_CROSS_SIZE: f32 = 7.0;
const CHEVRON_HALF_WIDTH: f32 = 4.0;
const CHEVRON_HALF_HEIGHT: f32 = 2.0;

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
