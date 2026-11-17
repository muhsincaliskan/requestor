use eframe::egui::Context;

use super::dialog;
use crate::state::{Action, AppState};

pub fn show(
    ctx: &Context,
    _state: &mut AppState,
    _collection_id: &str,
    _actions: &mut Vec<Action>,
) -> bool {
    dialog(ctx, "collection-editor", "Edit collection", |ui| {
        !ui.button("Close").clicked()
    })
}
