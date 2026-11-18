use eframe::egui::{CornerRadius, Frame, Margin, RichText, Ui};

use super::readout::{ByteSize, ElapsedText};
use crate::response::ResponseView;
use crate::widgets::status_color;

const TRUNCATED_LABEL: &str = "Truncated";
const TRUNCATED_HINT: &str = "The body exceeded the size limit and was cut off";
const SEPARATOR: &str = "·";
const BADGE_TINT: f32 = 0.16;
const BADGE_RADIUS: u8 = 10;
const BADGE_MARGIN: Margin = Margin::symmetric(8, 2);

pub fn show(ui: &mut Ui, view: &ResponseView) {
    ui.horizontal_wrapped(|ui| {
        let color = status_color(ui, view.status_class());
        let status = format!("{} {}", view.status, view.reason);
        Frame::new()
            .fill(color.gamma_multiply(BADGE_TINT))
            .corner_radius(CornerRadius::same(BADGE_RADIUS))
            .inner_margin(BADGE_MARGIN)
            .show(ui, |ui| {
                ui.label(RichText::new(status.trim()).color(color).strong())
            });
        ui.weak(SEPARATOR);
        ui.label(ElapsedText(view.elapsed).to_string());
        ui.weak(SEPARATOR);
        ui.label(ByteSize(view.size_bytes()).to_string());
        ui.weak(SEPARATOR);
        ui.weak(&view.version);
        if view.truncated {
            let warning = RichText::new(TRUNCATED_LABEL).color(ui.visuals().warn_fg_color);
            ui.label(warning).on_hover_text(TRUNCATED_HINT);
        }
    });
}
