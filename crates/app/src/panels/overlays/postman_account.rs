use crate::widgets::check_box;
use eframe::egui::{Button, ComboBox, Key, ProgressBar, RichText, TextEdit, Ui};

use super::TITLE_GAP;
use crate::state::{Action, MigrationForm, PostmanAccountForm};
use crate::widgets::action_button;

const HEADING: &str = "From your Postman account";
const EXPLANATION: &str = "Paste a Postman API key to import every collection and environment of a workspace, with folders kept. Create one in Postman under Settings, API keys.";
const KEY_HINT: &str = "PMAK-…";
const KEY_WIDTH: f32 = 340.0;
const LIST_LABEL: &str = "Load workspaces";
const REMEMBER_LABEL: &str = "Remember the key in the system keychain";
const LISTING_TEXT: &str = "Loading workspaces…";
const WORKSPACE_LABEL: &str = "Workspace";
const WORKSPACE_ID: &str = "postman-workspace";
const WORKSPACE_WIDTH: f32 = 300.0;
const IMPORT_LABEL: &str = "Import workspace";
const PREPARING_TEXT: &str = "Reading the workspace…";

pub fn show(
    ui: &mut Ui,
    form: &mut PostmanAccountForm,
    migration: &MigrationForm,
    actions: &mut Vec<Action>,
) {
    ui.label(RichText::new(HEADING).strong());
    ui.add_space(TITLE_GAP / 2.0);
    ui.weak(EXPLANATION);
    ui.add_space(TITLE_GAP);
    key_row(ui, form, actions);
    check_box(ui, &mut form.remember_key, REMEMBER_LABEL);
    if form.is_listing() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(LISTING_TEXT);
        });
    } else if let Some(error) = &form.error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
    if !form.workspaces.is_empty() {
        ui.add_space(TITLE_GAP);
        workspace_row(ui, form, migration, actions);
    }
    if migration.is_working() {
        progress(ui, form);
    }
}

fn key_row(ui: &mut Ui, form: &mut PostmanAccountForm, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        let field = TextEdit::singleline(&mut form.api_key)
            .password(true)
            .hint_text(KEY_HINT)
            .desired_width(KEY_WIDTH);
        let response = ui.add(field);
        if response.changed() {
            form.key_edited();
        }
        let submitted = response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
        let clicked = ui
            .add_enabled(form.can_list(), Button::new(LIST_LABEL))
            .clicked();
        if clicked || (submitted && form.can_list()) {
            actions.push(Action::ListPostmanWorkspaces);
        }
    });
}

fn workspace_row(
    ui: &mut Ui,
    form: &mut PostmanAccountForm,
    migration: &MigrationForm,
    actions: &mut Vec<Action>,
) {
    ui.horizontal(|ui| {
        ui.label(WORKSPACE_LABEL);
        let shown = form
            .selected_workspace()
            .map(|workspace| workspace.name.clone())
            .unwrap_or_default();
        ComboBox::from_id_salt(WORKSPACE_ID)
            .width(WORKSPACE_WIDTH)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for workspace in &form.workspaces {
                    let label = format!("{}  ·  {}", workspace.name, workspace.kind);
                    ui.selectable_value(&mut form.selected, Some(workspace.id.clone()), label);
                }
            });
    });
    let enabled = form.can_migrate() && !migration.is_working();
    let clicked = ui
        .add_enabled_ui(enabled, |ui| action_button::primary(ui, IMPORT_LABEL))
        .inner
        .clicked();
    if clicked {
        actions.push(Action::MigratePostmanWorkspace);
    }
}

fn progress(ui: &mut Ui, form: &PostmanAccountForm) {
    ui.add_space(TITLE_GAP);
    match &form.progress {
        Some(step) => {
            let share = step.done as f32 / step.total.max(1) as f32;
            ui.add(ProgressBar::new(share).text(format!(
                "{} of {} · {}",
                step.done + 1,
                step.total,
                step.current
            )));
        }
        None => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(PREPARING_TEXT);
            });
        }
    }
}
