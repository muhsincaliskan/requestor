use eframe::egui::{Context, ScrollArea, TextEdit, Ui};

use super::{button_row, dialog};
use crate::state::{Action, FolderPurpose, ImportForm};
use crate::widgets::action_button;

const DIALOG_ID: &str = "import";
const TITLE: &str = "Import";
const PASTE_ID: &str = "import-paste";
const SUPPORTED_TEXT: &str = "Postman collection v2.1, Postman environment, OpenAPI 3 JSON, WSDL 1.1, cURL command or raw HTTP request.";
const CHOOSE_FILE_LABEL: &str = "Choose file…";
const OPEN_FOLDER_LABEL: &str = "Open collection folder…";
const DROP_HINT: &str = "or drop a file anywhere on the window";
const PASTE_HINT: &str =
    "Paste text to import, or a WSDL address such as https://host/service?wsdl";
const DOWNLOADING_TEXT: &str = "Downloading…";
const IMPORT_TEXT_LABEL: &str = "Import";
const CANCEL_LABEL: &str = "Cancel";
const PASTE_HEIGHT: f32 = 200.0;
const PASTE_ROWS: usize = 8;
const DIALOG_WIDTH: f32 = 520.0;

pub fn show(ctx: &Context, form: &mut ImportForm, actions: &mut Vec<Action>) -> bool {
    dialog(ctx, DIALOG_ID, TITLE, |ui| {
        ui.set_width(DIALOG_WIDTH);
        ui.label(SUPPORTED_TEXT);
        file_row(ui, actions);
        paste_box(ui, &mut form.text);
        if form.is_downloading() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(DOWNLOADING_TEXT);
            });
        } else if let Some(error) = &form.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        button_row(ui, |ui| {
            text_button(ui, form, actions);
            !ui.button(CANCEL_LABEL).clicked()
        })
    })
}

fn paste_box(ui: &mut Ui, text: &mut String) {
    ScrollArea::vertical()
        .id_salt(PASTE_ID)
        .max_height(PASTE_HEIGHT)
        .show(ui, |ui| {
            ui.add(
                TextEdit::multiline(text)
                    .hint_text(PASTE_HINT)
                    .desired_rows(PASTE_ROWS)
                    .desired_width(f32::INFINITY),
            );
        });
}

fn text_button(ui: &mut Ui, form: &ImportForm, actions: &mut Vec<Action>) {
    let enabled = !form.text.trim().is_empty() && !form.is_downloading();
    let clicked = ui
        .add_enabled_ui(enabled, |ui| action_button::primary(ui, IMPORT_TEXT_LABEL))
        .inner
        .clicked();
    if clicked {
        actions.push(Action::ImportText(form.text.clone()));
    }
}

fn file_row(ui: &mut Ui, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        if ui.button(CHOOSE_FILE_LABEL).clicked() {
            actions.push(Action::ChooseImportFile);
        }
        if ui.button(OPEN_FOLDER_LABEL).clicked() {
            actions.push(Action::ChooseFolder(FolderPurpose::OpenCollection));
        }
        ui.weak(DROP_HINT);
    });
}
