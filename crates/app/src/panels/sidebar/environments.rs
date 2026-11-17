use eframe::egui::{Id, Label, RichText, Sense, Ui};

use super::tree::branch;
use super::{menus, row, row_label, trailing_add_button};
use crate::state::{Action, AppState};

const SECTION_ID: &str = "sidebar-environments";
const TITLE: &str = "Environments";
const NEW_ENVIRONMENT_LABEL: &str = "New environment";
const NO_ENVIRONMENT_LABEL: &str = "No environment";

pub(super) fn show(ui: &mut Ui, state: &AppState, actions: &mut Vec<Action>) {
    let mut add_clicked = false;
    branch(
        ui,
        Id::new(SECTION_ID),
        false,
        |ui| {
            let response = ui.add(Label::new(RichText::new(TITLE).strong()).sense(Sense::click()));
            add_clicked = trailing_add_button(ui, NEW_ENVIRONMENT_LABEL);
            response
        },
        |ui| rows(ui, state, actions),
    );
    if add_clicked {
        actions.push(Action::NewEnvironment);
    }
}

fn rows(ui: &mut Ui, state: &AppState, actions: &mut Vec<Action>) {
    let active = state.workspace.session.active_environment.as_deref();
    if ui
        .add(row(ui, active.is_none(), row_label(NO_ENVIRONMENT_LABEL)))
        .clicked()
    {
        actions.push(Action::SelectEnvironment(None));
    }
    for environment in &state.workspace.environments {
        let Some(id) = environment.id.as_deref() else {
            continue;
        };
        let response = ui.add(row(ui, active == Some(id), row_label(&environment.name)));
        if response.clicked() {
            actions.push(Action::SelectEnvironment(Some(id.to_string())));
        }
        menus::environment(&response, environment, actions);
    }
}
