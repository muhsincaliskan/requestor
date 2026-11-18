use eframe::egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, Ui};

use super::readout::{ByteSize, ElapsedText};
use crate::response::ResponseView;
use crate::state::{Action, TabId};
use crate::widgets::status_color;

const TRUNCATED_LABEL: &str = "Truncated";
const TRUNCATED_HINT: &str = "The body exceeded the size limit and was cut off";
const SEPARATOR: &str = "·";
const SAVE_EXAMPLE_LABEL: &str = "Save as example";
const SAVE_EXAMPLE_HINT: &str =
    "Keep this request and response under the saved request, as in Postman";
const BADGE_TINT: f32 = 0.16;
const BADGE_RADIUS: u8 = 10;
const BADGE_MARGIN: Margin = Margin::symmetric(8, 2);

pub fn show(
    ui: &mut Ui,
    view: &ResponseView,
    example_source: Option<TabId>,
    actions: &mut Vec<Action>,
) {
    ui.horizontal_wrapped(|ui| {
        let color = status_color(ui, view.status_class());
        let status = format!("{} {}", view.status, view.reason);
        badge(ui, status.trim(), color);
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
        if let Some(tab) = example_source {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let clicked = ui
                    .small_button(SAVE_EXAMPLE_LABEL)
                    .on_hover_text(SAVE_EXAMPLE_HINT)
                    .clicked();
                if clicked {
                    actions.push(Action::SaveExample(tab));
                }
            });
        }
    });
}

pub(super) fn badge(ui: &mut Ui, text: &str, color: Color32) {
    Frame::new()
        .fill(color.gamma_multiply(BADGE_TINT))
        .corner_radius(CornerRadius::same(BADGE_RADIUS))
        .inner_margin(BADGE_MARGIN)
        .show(ui, |ui| ui.label(RichText::new(text).color(color).strong()));
}
