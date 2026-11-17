use eframe::egui::{vec2, Button, Response, Ui};

const MIN_SIZE: f32 = 22.0;

pub fn icon_button(ui: &mut Ui, glyph: &str, hint: &str) -> Response {
    ui.add(
        Button::new(glyph)
            .frame(false)
            .min_size(vec2(MIN_SIZE, MIN_SIZE)),
    )
    .on_hover_text(hint)
}
