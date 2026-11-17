use eframe::egui::{Context, ScrollArea, TextEdit, Ui};

use super::dialog;
use crate::state::{Action, ImportForm};

const DIALOG_ID: &str = "import";
const TITLE: &str = "Import";
const PASTE_ID: &str = "import-paste";
const SUPPORTED_TEXT: &str = "Postman collection v2.1, Postman environment, OpenAPI 3 JSON, cURL command or raw HTTP request.";
const CHOOSE_FILE_LABEL: &str = "Choose file…";
const DROP_HINT: &str = "or drop a file anywhere on the window";
const PASTE_HINT: &str = "Paste a collection, environment, cURL command or raw HTTP request";
const IMPORT_TEXT_LABEL: &str = "Import pasted text";
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
        text_button(ui, form, actions);
        if let Some(error) = &form.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        !ui.button(CANCEL_LABEL).clicked()
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
    let enabled = !form.text.trim().is_empty();
    if ui
        .add_enabled(enabled, eframe::egui::Button::new(IMPORT_TEXT_LABEL))
        .clicked()
    {
        actions.push(Action::ImportText(form.text.clone()));
    }
}

fn file_row(ui: &mut Ui, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        if ui.button(CHOOSE_FILE_LABEL).clicked() {
            actions.push(Action::ChooseImportFile);
        }
        ui.weak(DROP_HINT);
    });
}
