use eframe::egui::{Align, ComboBox, Layout, Ui};

use crate::state::{Action, AppState, Overlay};

const NEW_LABEL: &str = "New";
const IMPORT_LABEL: &str = "Import";
const SETTINGS_LABEL: &str = "Settings";
const EDIT_ENVIRONMENT_LABEL: &str = "Edit";
const NO_ENVIRONMENT_LABEL: &str = "No environment";
const NEW_ENVIRONMENT_LABEL: &str = "New environment…";
const ENVIRONMENT_PICKER_ID: &str = "environment-picker";

pub fn show(ui: &mut Ui, state: &AppState, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        if ui.button(NEW_LABEL).clicked() {
            actions.push(Action::NewTab);
        }
        if ui.button(IMPORT_LABEL).clicked() {
            actions.push(Action::Show(Overlay::Import(Default::default())));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button(SETTINGS_LABEL).clicked() {
                actions.push(Action::Show(Overlay::Settings));
            }
            environment_picker(ui, state, actions);
        });
    });
}

fn environment_picker(ui: &mut Ui, state: &AppState, actions: &mut Vec<Action>) {
    let active = state.workspace.active_environment();
    let active_id = active.and_then(|environment| environment.id.clone());
    if ui
        .add_enabled(
            active_id.is_some(),
            eframe::egui::Button::new(EDIT_ENVIRONMENT_LABEL),
        )
        .clicked()
    {
        actions.extend(
            active_id
                .clone()
                .map(|environment_id| Action::Show(Overlay::EnvironmentEditor { environment_id })),
        );
    }
    let selected = active.map_or(NO_ENVIRONMENT_LABEL, |environment| {
        environment.name.as_str()
    });
    ComboBox::from_id_salt(ENVIRONMENT_PICKER_ID)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(active_id.is_none(), NO_ENVIRONMENT_LABEL)
                .clicked()
            {
                actions.push(Action::SelectEnvironment(None));
            }
            for environment in &state.workspace.environments {
                let chosen = environment.id.is_some() && environment.id == active_id;
                if ui.selectable_label(chosen, &environment.name).clicked() {
                    actions.push(Action::SelectEnvironment(environment.id.clone()));
                }
            }
            ui.separator();
            if ui.button(NEW_ENVIRONMENT_LABEL).clicked() {
                actions.push(Action::NewEnvironment);
            }
        });
}
