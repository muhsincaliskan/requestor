use eframe::egui::{vec2, Align, ComboBox, Layout, Rect, Ui};
use requestor_core::environment::Environment;

use crate::state::{Action, AppState, Overlay};
use crate::widgets::{environment_color, environment_swatch, PROTECTED_SEND_HINT};

const NEW_LABEL: &str = "New";
const IMPORT_LABEL: &str = "Import";
const SETTINGS_LABEL: &str = "Settings";
const EDIT_ENVIRONMENT_LABEL: &str = "Edit";
const EDIT_ENVIRONMENT_HINT: &str = "Edit the active environment";
const NO_ENVIRONMENT_LABEL: &str = "No environment";
const NEW_ENVIRONMENT_LABEL: &str = "New environment…";
const ENVIRONMENT_PICKER_ID: &str = "environment-picker";
const STRIP_HEIGHT: f32 = 3.0;

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
    if let Some(environment_id) = active_id.clone() {
        if ui
            .button(EDIT_ENVIRONMENT_LABEL)
            .on_hover_text(EDIT_ENVIRONMENT_HINT)
            .clicked()
        {
            actions.push(Action::Show(Overlay::EnvironmentEditor { environment_id }));
        }
    }
    let selected = active.map_or(NO_ENVIRONMENT_LABEL, |environment| {
        environment.name.as_str()
    });
    let picker = ComboBox::from_id_salt(ENVIRONMENT_PICKER_ID)
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
                if environment_choice(ui, environment, chosen) {
                    actions.push(Action::SelectEnvironment(environment.id.clone()));
                }
            }
            ui.separator();
            if ui.button(NEW_ENVIRONMENT_LABEL).clicked() {
                actions.push(Action::NewEnvironment);
            }
        });
    if let Some(environment) = active {
        marker(ui, environment);
    }
    if active.is_some_and(Environment::is_protected) {
        picker.response.on_hover_text(PROTECTED_SEND_HINT);
    }
}

pub fn environment_strip(ui: &Ui, toolbar: Rect, state: &AppState) {
    let Some(color) = state
        .workspace
        .active_environment()
        .and_then(Environment::color)
    else {
        return;
    };
    let strip = Rect::from_min_max(
        toolbar.left_bottom() - vec2(0.0, STRIP_HEIGHT),
        toolbar.right_bottom(),
    );
    ui.painter()
        .rect_filled(strip, 0.0, environment_color(ui, color));
}

fn marker(ui: &mut Ui, environment: &Environment) {
    if let Some(color) = environment.color() {
        environment_swatch(ui, color);
    }
}

fn environment_choice(ui: &mut Ui, environment: &Environment, chosen: bool) -> bool {
    ui.horizontal(|ui| {
        let clicked = ui.selectable_label(chosen, &environment.name).clicked();
        marker(ui, environment);
        clicked
    })
    .inner
}
