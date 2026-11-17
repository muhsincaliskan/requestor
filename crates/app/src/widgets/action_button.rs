use eframe::egui::{Button, Color32, Response, RichText, Ui};

use crate::theme::label_on;

pub fn primary(ui: &mut Ui, label: &str) -> Response {
    let fill = ui.visuals().hyperlink_color;
    filled(ui, label, fill)
}

pub fn destructive(ui: &mut Ui, label: &str) -> Response {
    let fill = ui.visuals().error_fg_color;
    filled(ui, label, fill)
}

fn filled(ui: &mut Ui, label: &str, fill: Color32) -> Response {
    ui.add(Button::new(RichText::new(label).color(label_on(fill)).strong()).fill(fill))
}
