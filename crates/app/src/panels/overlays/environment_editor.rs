use eframe::egui::Context;

use super::dialog;
use crate::state::{Action, AppState};

pub fn show(
    ctx: &Context,
    _state: &mut AppState,
    _environment_id: &str,
    _actions: &mut Vec<Action>,
) -> bool {
    dialog(ctx, "environment-editor", "Edit environment", |ui| {
        !ui.button("Close").clicked()
    })
}
