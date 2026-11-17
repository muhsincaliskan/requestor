use eframe::egui::{Id, Label, RichText, Sense, Ui};

use super::menus;
use super::tree::branch;
use crate::state::{Action, AppState};

const SECTION_ID: &str = "sidebar-environments";
const TITLE: &str = "Environments";
const NEW_LABEL: &str = "+";
const NO_ENVIRONMENT_LABEL: &str = "No environment";

pub(super) fn show(ui: &mut Ui, state: &AppState, actions: &mut Vec<Action>) {
    let mut add_clicked = false;
    branch(
        ui,
        Id::new(SECTION_ID),
        false,
        |ui| {
            let response = ui.add(Label::new(RichText::new(TITLE).strong()).sense(Sense::click()));
            add_clicked = ui.small_button(NEW_LABEL).clicked();
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
        .selectable_label(active.is_none(), NO_ENVIRONMENT_LABEL)
        .clicked()
    {
        actions.push(Action::SelectEnvironment(None));
    }
    for environment in &state.workspace.environments {
        let Some(id) = environment.id.as_deref() else {
            continue;
        };
        let response = ui.selectable_label(active == Some(id), &environment.name);
        if response.clicked() {
            actions.push(Action::SelectEnvironment(Some(id.to_string())));
        }
        menus::environment(&response, environment, actions);
    }
}
