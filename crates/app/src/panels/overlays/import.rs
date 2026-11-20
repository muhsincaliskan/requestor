use eframe::egui::{Context, RichText, ScrollArea, TextEdit, Ui};

use super::{button_row, dialog, TITLE_GAP};
use crate::state::{Action, FolderPurpose, ImportForm, Overlay};
use crate::widgets::action_button;

const DIALOG_ID: &str = "import";
const TITLE: &str = "Import";
const PASTE_ID: &str = "import-paste";
const SUPPORTED_TEXT: &str =
    "Supports Postman collections v2.1 and environments, OpenAPI 3 JSON, WSDL 1.1, cURL commands and raw HTTP requests.";
const FILE_HEADING: &str = "From a file";
const PASTE_HEADING: &str = "Or paste";
const CHOOSE_FILE_LABEL: &str = "Choose file…";
const OPEN_FOLDER_LABEL: &str = "Open collection folder…";
const DROP_HINT: &str = "You can also drop a file anywhere on the window.";
const PASTE_HINT: &str =
    "Paste text to import, or a WSDL address such as https://host/service?wsdl";
const DOWNLOADING_TEXT: &str = "Downloading…";
const IMPORT_TEXT_LABEL: &str = "Import";
const CANCEL_LABEL: &str = "Cancel";
const MIGRATION_TEXT: &str = "Moving a whole workspace?";
const MIGRATION_LABEL: &str = "Import from Postman…";
const PASTE_HEIGHT: f32 = 200.0;
const PASTE_ROWS: usize = 8;
const DIALOG_WIDTH: f32 = 560.0;
const SECTION_GAP: f32 = 16.0;

pub fn show(ctx: &Context, form: &mut ImportForm, actions: &mut Vec<Action>) -> bool {
    dialog(ctx, DIALOG_ID, TITLE, |ui| {
        ui.set_width(DIALOG_WIDTH);
        section_heading(ui, FILE_HEADING);
        file_row(ui, actions);
        ui.weak(DROP_HINT);
        migration_row(ui, actions);
        ui.add_space(SECTION_GAP);
        section_heading(ui, PASTE_HEADING);
        paste_box(ui, &mut form.text);
        ui.add_space(TITLE_GAP);
        ui.label(RichText::new(SUPPORTED_TEXT).weak().small());
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
    });
}

fn section_heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).strong());
    ui.add_space(TITLE_GAP / 2.0);
}

fn migration_row(ui: &mut Ui, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.weak(MIGRATION_TEXT);
        if ui.link(MIGRATION_LABEL).clicked() {
            actions.push(Action::Show(Overlay::PostmanMigration));
        }
    });
}
