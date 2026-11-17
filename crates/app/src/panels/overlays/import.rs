use eframe::egui::Context;

use super::dialog;
use crate::state::{Action, ImportForm};

pub fn show(ctx: &Context, _form: &mut ImportForm, _actions: &mut Vec<Action>) -> bool {
    dialog(ctx, "import", "Import", |ui| !ui.button("Close").clicked())
}
