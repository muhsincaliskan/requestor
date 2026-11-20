use eframe::egui::{Align, Layout, Popup, Ui};

use super::NEW_LABEL;
use crate::state::Action;
use crate::widgets::icon_button;

const ADD_HINT: &str = "Add a request or folder";
const ADD_REQUEST_LABEL: &str = "Add request";
const ADD_FOLDER_LABEL: &str = "Add folder";

pub(super) fn show(ui: &mut Ui, collection_id: &str, parent_id: Option<&str>) -> Option<Action> {
    let mut chosen = None;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let button = icon_button(ui, NEW_LABEL, ADD_HINT);
        Popup::menu(&button).show(|ui| {
            if ui.button(ADD_REQUEST_LABEL).clicked() {
                chosen = Some(Action::AddRequest {
                    collection_id: collection_id.to_string(),
                    parent_id: parent_id.map(str::to_string),
                });
            }
            if ui.button(ADD_FOLDER_LABEL).clicked() {
                chosen = Some(Action::AddFolder {
                    collection_id: collection_id.to_string(),
                    parent_id: parent_id.map(str::to_string),
                });
            }
        });
    });
    chosen
}
