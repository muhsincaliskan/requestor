use eframe::egui::Context;

use super::{button_row, dialog};
use crate::state::{Action, AppState, TabId};
use crate::widgets::action_button;

const DIALOG_ID: &str = "confirm-protected-send";
const SEND_LABEL: &str = "Send";
const CANCEL_LABEL: &str = "Cancel";
const EXPLANATION: &str = "This environment is protected. Requests sent here may change real data.";
const PROTECTION_HINT: &str = "You can turn this question off in the environment's settings.";

pub fn show(ctx: &Context, state: &AppState, tab: TabId, actions: &mut Vec<Action>) -> bool {
    let (Some(open), Some(environment)) = (state.tab(tab), state.workspace.active_environment())
    else {
        return false;
    };
    let title = format!("Send \"{}\" to {}?", open.draft.name, environment.name);
    dialog(ctx, DIALOG_ID, &title, |ui| {
        ui.label(EXPLANATION);
        ui.weak(PROTECTION_HINT);
        button_row(ui, |ui| {
            if action_button::destructive(ui, SEND_LABEL).clicked() {
                actions.push(Action::SendConfirmed(tab));
                return false;
            }
            !ui.button(CANCEL_LABEL).clicked()
        })
    })
}
