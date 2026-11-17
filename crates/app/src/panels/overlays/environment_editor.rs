use eframe::egui::{Context, Id, Ui};
use requestor_core::environment::EnvironmentValue;

use super::dialog;
use super::table::{
    delete_cell, edit_rows, enabled_cell, footer, name_field, text_cell, RowEdit, KEY_HINT,
    VALUE_HINT,
};
use crate::state::{Action, AppState};

const DIALOG_ID: &str = "environment-editor";
const TABLE_ID: &str = "environment-values";
const REVEAL_ID: &str = "environment-reveal-secrets";
const TITLE: &str = "Edit environment";
const SECRET_LABEL: &str = "Secret";
const REVEAL_LABEL: &str = "Show secret values";
const COLUMNS: usize = 5;

pub fn show(
    ctx: &Context,
    state: &mut AppState,
    environment_id: &str,
    actions: &mut Vec<Action>,
) -> bool {
    let Some(environment) = state.workspace.environment_mut(environment_id) else {
        return false;
    };
    let stays_open = dialog(ctx, DIALOG_ID, TITLE, |ui| {
        let mut changed = name_field(ui, &mut environment.name);
        let reveal = reveal_toggle(ui);
        changed |= edit_rows(
            ui,
            TABLE_ID,
            COLUMNS,
            &mut environment.values,
            |ui, entry| row(ui, entry, reveal),
        );
        let (add, done) = footer(ui);
        if add {
            environment.values.push(EnvironmentValue::blank());
            changed = true;
        }
        if changed {
            actions.push(Action::EnvironmentEdited(environment_id.to_string()));
        }
        !done
    });
    if !stays_open {
        ctx.data_mut(|data| data.remove_temp::<bool>(Id::new(REVEAL_ID)));
    }
    stays_open
}

fn reveal_toggle(ui: &mut Ui) -> bool {
    let id = Id::new(REVEAL_ID);
    let mut reveal = ui.data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
    if ui.checkbox(&mut reveal, REVEAL_LABEL).changed() {
        ui.data_mut(|data| data.insert_temp(id, reveal));
    }
    reveal
}

fn row(ui: &mut Ui, entry: &mut EnvironmentValue, reveal: bool) -> RowEdit {
    let mut changed = false;
    if let Some(enabled) = enabled_cell(ui, entry.is_enabled()) {
        entry.set_enabled(enabled);
        changed = true;
    }
    changed |= text_cell(ui, &mut entry.key, KEY_HINT, false);
    changed |= value_cell(ui, entry, reveal);
    changed |= secret_cell(ui, entry);
    RowEdit::from_flags(changed, delete_cell(ui))
}

fn value_cell(ui: &mut Ui, entry: &mut EnvironmentValue, reveal: bool) -> bool {
    let mut text = entry.value_text();
    let masked = entry.is_secret() && !reveal;
    let edited = text_cell(ui, &mut text, VALUE_HINT, masked);
    if edited {
        entry.set_value_text(&text);
    }
    edited
}

fn secret_cell(ui: &mut Ui, entry: &mut EnvironmentValue) -> bool {
    let mut secret = entry.is_secret();
    let toggled = ui.checkbox(&mut secret, SECRET_LABEL).changed();
    if toggled {
        entry.set_secret(secret);
    }
    toggled
}
