use eframe::egui::{Context, Ui};
use requestor_core::collection::{Collection, Variable};

use super::dialog;
use super::table::{
    delete_cell, edit_rows, enabled_cell, footer, forget_reveal, name_field, reveal_toggle,
    text_cell, RowEdit, KEY_HINT, VALUE_HINT,
};
use crate::state::{Action, AppState};

const DIALOG_ID: &str = "collection-editor";
const TABLE_ID: &str = "collection-variables";
const REVEAL_ID: &str = "collection-reveal-secrets";
const TITLE: &str = "Edit collection";
const VARIABLES_LABEL: &str = "Variables";
const AUTH_LABEL: &str = "Authorization";
const NO_AUTH_TEXT: &str = "none";
const COLUMNS: usize = 4;

pub fn show(
    ctx: &Context,
    state: &mut AppState,
    collection_id: &str,
    actions: &mut Vec<Action>,
) -> bool {
    let Some(collection) = state.workspace.collection_mut(collection_id) else {
        return false;
    };
    let stays_open = dialog(ctx, DIALOG_ID, TITLE, |ui| {
        let mut changed = name_field(ui, &mut collection.info.name);
        auth_line(ui, collection);
        ui.strong(VARIABLES_LABEL);
        changed |= variables_table(ui, collection);
        let (add, done) = footer(ui);
        if add {
            collection
                .variable
                .get_or_insert_default()
                .push(Variable::text("", ""));
            changed = true;
        }
        if changed {
            actions.push(Action::CollectionEdited(collection_id.to_string()));
        }
        !done
    });
    if !stays_open {
        forget_reveal(ctx, REVEAL_ID);
    }
    stays_open
}

fn auth_line(ui: &mut Ui, collection: &Collection) {
    let kind = collection
        .auth
        .as_ref()
        .map_or(NO_AUTH_TEXT, |auth| auth.kind.as_str());
    ui.weak(format!("{AUTH_LABEL}: {kind}"));
}

fn variables_table(ui: &mut Ui, collection: &mut Collection) -> bool {
    let Some(variables) = collection.variable.as_mut() else {
        return false;
    };
    let reveal = variables.iter().any(Variable::is_secret) && reveal_toggle(ui, REVEAL_ID);
    edit_rows(ui, TABLE_ID, COLUMNS, variables, |ui, variable| {
        row(ui, variable, reveal)
    })
}

fn row(ui: &mut Ui, variable: &mut Variable, reveal: bool) -> RowEdit {
    let mut changed = false;
    if let Some(enabled) = enabled_cell(ui, variable.is_enabled()) {
        variable.set_enabled(enabled);
        changed = true;
    }
    let mut key = variable.key_text().to_string();
    if text_cell(ui, &mut key, KEY_HINT, false) {
        variable.set_key_text(&key);
        changed = true;
    }
    let mut value = variable.value_text();
    let masked = variable.is_secret() && !reveal;
    if text_cell(ui, &mut value, VALUE_HINT, masked) {
        variable.set_value_text(&value);
        changed = true;
    }
    RowEdit::from_flags(changed, delete_cell(ui))
}
