use eframe::egui::{vec2, Button, Response, Ui};

pub const ICON_BUTTON_SIZE: f32 = 22.0;

pub fn icon_button(ui: &mut Ui, glyph: &str, hint: &str) -> Response {
    ui.add(
        Button::new(glyph)
            .frame(false)
            .min_size(vec2(ICON_BUTTON_SIZE, ICON_BUTTON_SIZE)),
    )
    .on_hover_text(hint)
}
