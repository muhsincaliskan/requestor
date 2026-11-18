use eframe::egui::{Color32, Ui};
use requestor_core::response::StatusClass;

const SUCCESS_ON_DARK: Color32 = Color32::from_rgb(107, 203, 119);
const SUCCESS_ON_LIGHT: Color32 = Color32::from_rgb(0, 112, 43);

pub fn success_color(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        SUCCESS_ON_DARK
    } else {
        SUCCESS_ON_LIGHT
    }
}

pub fn status_color(ui: &Ui, class: StatusClass) -> Color32 {
    match class {
        StatusClass::Success => success_color(ui),
        StatusClass::ClientError | StatusClass::ServerError => ui.visuals().error_fg_color,
        StatusClass::Redirect => ui.visuals().hyperlink_color,
        StatusClass::Informational | StatusClass::Unknown => ui.visuals().text_color(),
    }
}
