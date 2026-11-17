use eframe::egui::{Color32, RichText, Ui};
use requestor_core::response::StatusClass;

use super::readout::{ByteSize, ElapsedText};
use crate::response::ResponseView;

const SUCCESS_ON_DARK: Color32 = Color32::from_rgb(107, 203, 119);
const SUCCESS_ON_LIGHT: Color32 = Color32::from_rgb(0, 127, 49);
const TRUNCATED_LABEL: &str = "Truncated";
const TRUNCATED_HINT: &str = "The body exceeded the size limit and was cut off";
const SEPARATOR: &str = "·";

fn class_color(ui: &Ui, class: StatusClass) -> Color32 {
    match class {
        StatusClass::Success if ui.visuals().dark_mode => SUCCESS_ON_DARK,
        StatusClass::Success => SUCCESS_ON_LIGHT,
        StatusClass::ClientError | StatusClass::ServerError => ui.visuals().error_fg_color,
        StatusClass::Informational | StatusClass::Redirect | StatusClass::Unknown => {
            ui.visuals().text_color()
        }
    }
}

pub fn show(ui: &mut Ui, view: &ResponseView) {
    ui.horizontal_wrapped(|ui| {
        let color = class_color(ui, view.status_class());
        let status = format!("{} {}", view.status, view.reason);
        ui.label(RichText::new(status.trim()).color(color).strong());
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
