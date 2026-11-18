use eframe::egui::{Id, Label, RichText, Sense, Ui};

use super::tree::branch;
use super::{menus, trailing_add_button};
use crate::state::{Action, AppState};
use crate::widgets::{list_row, row_label};

const SECTION_ID: &str = "sidebar-environments";
const TITLE: &str = "Environments";
const NEW_ENVIRONMENT_LABEL: &str = "New environment";
const EMPTY_TEXT: &str = "No environments yet.";
const ACTIVATE_HINT: &str = "Click to use this environment";
const DEACTIVATE_HINT: &str = "In use. Click to stop using it";

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
    if state.workspace.environments.is_empty() {
        ui.weak(EMPTY_TEXT);
    }
    for environment in &state.workspace.environments {
        let Some(id) = environment.id.as_deref() else {
            continue;
        };
        let in_use = active == Some(id);
        let response = ui
            .add(list_row(ui, in_use, row_label(&environment.name)))
            .on_hover_text(if in_use {
                DEACTIVATE_HINT
            } else {
                ACTIVATE_HINT
            });
        if response.clicked() {
            let next = (!in_use).then(|| id.to_string());
            actions.push(Action::SelectEnvironment(next));
        }
        menus::environment(&response, environment, actions);
    }
}
