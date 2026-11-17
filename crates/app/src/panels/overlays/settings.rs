use eframe::egui::Context;

use super::dialog;
use crate::state::{Action, AppState};

pub fn show(ctx: &Context, _state: &mut AppState, _actions: &mut Vec<Action>) -> bool {
    dialog(ctx, "settings", "Settings", |ui| {
        !ui.button("Close").clicked()
    })
}
