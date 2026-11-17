use eframe::egui::{Context, TextEdit, Ui};
use requestor_core::collection::{Collection, Variable};

use super::dialog;
use super::table::{
    delete_cell, edit_rows, enabled_cell, text_cell, RowEdit, ADD_VARIABLE_LABEL, DONE_LABEL,
    KEY_HINT, NAME_LABEL, VALUE_HINT,
};
use crate::state::{Action, AppState};

const DIALOG_ID: &str = "collection-editor";
const TABLE_ID: &str = "collection-variables";
const TITLE: &str = "Edit collection";
const VARIABLES_LABEL: &str = "Variables";
const AUTH_LABEL: &str = "Authorization";
const NO_AUTH_TEXT: &str = "none";
const COLUMNS: usize = 4;
const NAME_WIDTH: f32 = 320.0;

pub fn show(
    ctx: &Context,
    state: &mut AppState,
    collection_id: &str,
    actions: &mut Vec<Action>,
) -> bool {
    let Some(collection) = state.workspace.collection_mut(collection_id) else {
        return false;
    };
    dialog(ctx, DIALOG_ID, TITLE, |ui| {
        let mut changed = name_field(ui, &mut collection.info.name);
        auth_line(ui, collection);
        ui.strong(VARIABLES_LABEL);
        changed |= variables_table(ui, collection);
        if ui.button(ADD_VARIABLE_LABEL).clicked() {
            collection
                .variable
                .get_or_insert_default()
                .push(Variable::text("", ""));
            changed = true;
        }
        if changed {
            actions.push(Action::CollectionEdited(collection_id.to_string()));
        }
        !ui.button(DONE_LABEL).clicked()
    })
}

fn name_field(ui: &mut Ui, name: &mut String) -> bool {
    ui.horizontal(|ui| {
        ui.label(NAME_LABEL);
        ui.add(TextEdit::singleline(name).desired_width(NAME_WIDTH))
            .changed()
    })
    .inner
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
    edit_rows(ui, TABLE_ID, COLUMNS, variables, row)
}

fn row(ui: &mut Ui, variable: &mut Variable) -> RowEdit {
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
    if text_cell(ui, &mut value, VALUE_HINT, false) {
        variable.set_value_text(&value);
        changed = true;
    }
    RowEdit::from_flags(changed, delete_cell(ui))
}
