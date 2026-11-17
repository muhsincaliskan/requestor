mod drag;
mod environments;
mod menus;
mod tree;

use eframe::egui::{Id, ScrollArea, TextEdit, Ui};

use crate::state::{Action, AppState, Overlay};

const TITLE: &str = "Collections";
const NEW_LABEL: &str = "+";
const IMPORT_LABEL: &str = "Import";
const NEW_COLLECTION_LABEL: &str = "New collection";
const FILTER_HINT: &str = "Filter requests";
const FILTER_ID: &str = "sidebar-filter";
const SCROLL_ID: &str = "sidebar-scroll";
const EMPTY_TEXT: &str =
    "No collections yet. Create one or import from Postman, OpenAPI, cURL or a raw HTTP request.";
const SECTION_GAP: f32 = 8.0;

pub fn show(ui: &mut Ui, state: &mut AppState, actions: &mut Vec<Action>) {
    header(ui, actions);
    let filter = filter_field(ui);
    ui.add_space(SECTION_GAP);
    ScrollArea::vertical()
        .id_salt(SCROLL_ID)
        .auto_shrink(false)
        .show(ui, |ui| {
            if state.workspace.collections.is_empty() {
                empty_state(ui, actions);
            } else {
                tree::show(ui, state, &filter, actions);
            }
            ui.add_space(SECTION_GAP);
            environments::show(ui, state, actions);
        });
}

fn header(ui: &mut Ui, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.strong(TITLE);
        ui.with_layout(
            eframe::egui::Layout::right_to_left(eframe::egui::Align::Center),
            |ui| {
                if ui.button(NEW_LABEL).clicked() {
                    actions.push(Action::NewCollection);
                }
                if ui.button(IMPORT_LABEL).clicked() {
                    actions.push(Action::Show(Overlay::Import(Default::default())));
                }
            },
        );
    });
}

fn filter_field(ui: &mut Ui) -> String {
    let id = Id::new(FILTER_ID);
    let mut text: String = ui.data(|data| data.get_temp(id)).unwrap_or_default();
    let edit = TextEdit::singleline(&mut text)
        .hint_text(FILTER_HINT)
        .desired_width(f32::INFINITY);
    if ui.add(edit).changed() {
        ui.data_mut(|data| data.insert_temp(id, text.clone()));
    }
    text.trim().to_string()
}

fn empty_state(ui: &mut Ui, actions: &mut Vec<Action>) {
    ui.weak(EMPTY_TEXT);
    ui.horizontal(|ui| {
        if ui.button(NEW_COLLECTION_LABEL).clicked() {
            actions.push(Action::NewCollection);
        }
        if ui.button(IMPORT_LABEL).clicked() {
            actions.push(Action::Show(Overlay::Import(Default::default())));
        }
    });
}
