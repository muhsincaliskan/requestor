use crate::widgets::check_box;
use eframe::egui::{Context, RichText, ScrollArea, Ui};

use super::postman_account;
use super::{button_row, dialog, TITLE_GAP};
use crate::state::{Action, FolderPurpose, MigrationForm, MigrationOutcome, PostmanAccountForm};

const DIALOG_ID: &str = "postman-migration";
pub const TITLE: &str = "Import from Postman";
const DIALOG_WIDTH: f32 = 560.0;
const SECTION_GAP: f32 = 16.0;
const DETAILS_HEIGHT: f32 = 200.0;
const DETAILS_ID: &str = "postman-migration-details";
const FILES_HEADING: &str = "From Postman exports";
const FILES_TEXT: &str = "Pick collection and environment files, a folder of them, or the zip from Postman's \"Export data\". Everything is imported at once, with folders kept.";
const CHOOSE_FILES_LABEL: &str = "Choose files…";
const CHOOSE_FOLDER_LABEL: &str = "Choose folder…";
const DROP_HINT: &str = "You can also drop files or folders anywhere on the window.";
const WORKING_TEXT: &str = "Importing…";
const FAILURES_HEADING: &str = "Not imported";
const WARNINGS_HEADING: &str = "Warnings";
const CLOSE_LABEL: &str = "Close";
const EXPORT_HEADING: &str = "Back to Postman";
const EXPORT_TEXT: &str = "Export every collection and environment as one zip of Postman files.";
const EXPORT_LABEL: &str = "Export workspace as zip…";
const CREDENTIALS_LABEL: &str = "Include credentials";
const CREDENTIALS_HINT: &str = "Keeps tokens and passwords written into requests and environments. Values marked as secret are always left out.";

pub fn show(
    ctx: &Context,
    account: &mut PostmanAccountForm,
    form: &mut MigrationForm,
    actions: &mut Vec<Action>,
) -> bool {
    dialog(ctx, DIALOG_ID, TITLE, |ui| {
        ui.set_width(DIALOG_WIDTH);
        postman_account::show(ui, account, form, actions);
        ui.add_space(SECTION_GAP);
        ui.separator();
        ui.add_space(SECTION_GAP);
        files_section(ui, form, actions);
        ui.add_space(SECTION_GAP);
        ui.separator();
        ui.add_space(SECTION_GAP);
        export_section(ui, form, actions);
        ui.add_space(SECTION_GAP);
        if form.is_working() && account.progress.is_none() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(WORKING_TEXT);
            });
        } else if let Some(outcome) = &form.outcome {
            outcome_section(ui, outcome);
        }
        button_row(ui, |ui| !ui.button(CLOSE_LABEL).clicked())
    })
}

fn files_section(ui: &mut Ui, form: &MigrationForm, actions: &mut Vec<Action>) {
    section_heading(ui, FILES_HEADING);
    ui.weak(FILES_TEXT);
    ui.add_space(TITLE_GAP);
    ui.add_enabled_ui(!form.is_working(), |ui| {
        ui.horizontal(|ui| {
            if ui.button(CHOOSE_FILES_LABEL).clicked() {
                actions.push(Action::ChooseImportFiles);
            }
            if ui.button(CHOOSE_FOLDER_LABEL).clicked() {
                actions.push(Action::ChooseFolder(FolderPurpose::PostmanExport));
            }
        });
    });
    ui.label(RichText::new(DROP_HINT).weak().small());
}

fn export_section(ui: &mut Ui, form: &mut MigrationForm, actions: &mut Vec<Action>) {
    section_heading(ui, EXPORT_HEADING);
    ui.weak(EXPORT_TEXT);
    ui.add_space(TITLE_GAP);
    ui.horizontal(|ui| {
        if ui.button(EXPORT_LABEL).clicked() {
            actions.push(Action::ChooseWorkspaceArchiveLocation);
        }
        check_box(ui, &mut form.include_credentials, CREDENTIALS_LABEL)
            .on_hover_text(CREDENTIALS_HINT);
    });
}

fn outcome_section(ui: &mut Ui, outcome: &MigrationOutcome) {
    ui.label(&outcome.summary);
    if outcome.failures.is_empty() && outcome.warnings.is_empty() {
        return;
    }
    ui.add_space(TITLE_GAP);
    ScrollArea::vertical()
        .id_salt(DETAILS_ID)
        .max_height(DETAILS_HEIGHT)
        .show(ui, |ui| {
            if !outcome.failures.is_empty() {
                section_heading(ui, FAILURES_HEADING);
            }
            for failure in &outcome.failures {
                ui.label(RichText::new(&failure.source).strong());
                ui.colored_label(ui.visuals().error_fg_color, &failure.reason);
            }
            if !outcome.warnings.is_empty() {
                section_heading(ui, WARNINGS_HEADING);
            }
            for warning in &outcome.warnings {
                ui.colored_label(ui.visuals().warn_fg_color, warning);
            }
        });
}

fn section_heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).strong());
    ui.add_space(TITLE_GAP / 2.0);
}
