use eframe::egui::{ComboBox, Grid, Id, Ui};
use requestor_core::draft::{AuthDraft, AuthKind};

use super::variable_text_edit::{VariableHints, VariableTextEdit};

const KIND_SALT: &str = "auth-kind";
const GRID_SALT: &str = "auth-fields";
const REVEAL_SALT: &str = "reveal";
const INHERIT_HINT: &str =
    "This request uses the authorization of its parent folder or collection.";
const NONE_HINT: &str = "This request is sent without authorization.";
const UNSUPPORTED_NOTE: &str =
    "This auth type is preserved when saving, but it cannot be edited here:";
const KIND_LABEL: &str = "Type";
const TOKEN_LABEL: &str = "Token";
const USERNAME_LABEL: &str = "Username";
const PASSWORD_LABEL: &str = "Password";
const KEY_NAME_LABEL: &str = "Key";
const KEY_VALUE_LABEL: &str = "Value";
const ADD_TO_LABEL: &str = "Add to";
const HEADER_LABEL: &str = "Header";
const QUERY_LABEL: &str = "Query params";
const SHOW_LABEL: &str = "Show";
const HIDE_LABEL: &str = "Hide";
const FIELD_WIDTH: f32 = 320.0;

pub fn show(ui: &mut Ui, auth: &mut AuthDraft, hints: &VariableHints, id_salt: &str) -> bool {
    let id = ui.id().with(id_salt);
    let mut changed = kind_picker(ui, auth, id);
    if let Some(kind) = auth.unsupported_kind() {
        ui.label(UNSUPPORTED_NOTE);
        ui.monospace(kind);
        return changed;
    }
    ui.add_space(ui.spacing().item_spacing.y);
    changed |= match auth.kind {
        AuthKind::Inherit => hint(ui, INHERIT_HINT),
        AuthKind::None => hint(ui, NONE_HINT),
        AuthKind::Bearer => bearer_fields(ui, auth, hints, id),
        AuthKind::Basic => basic_fields(ui, auth, hints, id),
        AuthKind::ApiKey => api_key_fields(ui, auth, hints, id),
        AuthKind::Unsupported => false,
    };
    changed
}

fn hint(ui: &mut Ui, text: &str) -> bool {
    ui.weak(text);
    false
}

fn kind_picker(ui: &mut Ui, auth: &mut AuthDraft, id: Id) -> bool {
    let before = auth.kind;
    ui.horizontal(|ui| {
        ui.label(KIND_LABEL);
        ComboBox::from_id_salt(id.with(KIND_SALT))
            .selected_text(auth.kind.label())
            .show_ui(ui, |ui| {
                for kind in AuthKind::SELECTABLE {
                    ui.selectable_value(&mut auth.kind, kind, kind.label());
                }
            });
    });
    auth.kind != before
}

fn bearer_fields(ui: &mut Ui, auth: &mut AuthDraft, hints: &VariableHints, id: Id) -> bool {
    fields_grid(ui, id, |ui| {
        secret_row(ui, TOKEN_LABEL, &mut auth.token, hints, id)
    })
}

fn basic_fields(ui: &mut Ui, auth: &mut AuthDraft, hints: &VariableHints, id: Id) -> bool {
    fields_grid(ui, id, |ui| {
        let user = text_row(ui, USERNAME_LABEL, &mut auth.username, hints, id);
        user | secret_row(ui, PASSWORD_LABEL, &mut auth.password, hints, id)
    })
}

fn api_key_fields(ui: &mut Ui, auth: &mut AuthDraft, hints: &VariableHints, id: Id) -> bool {
    fields_grid(ui, id, |ui| {
        let name = text_row(ui, KEY_NAME_LABEL, &mut auth.api_key_name, hints, id);
        let value = secret_row(ui, KEY_VALUE_LABEL, &mut auth.api_key_value, hints, id);
        name | value | location_row(ui, &mut auth.api_key_in_query)
    })
}

fn fields_grid(ui: &mut Ui, id: Id, add_rows: impl FnOnce(&mut Ui) -> bool) -> bool {
    Grid::new(id.with(GRID_SALT))
        .num_columns(2)
        .show(ui, add_rows)
        .inner
}

fn text_row(ui: &mut Ui, label: &str, value: &mut String, hints: &VariableHints, id: Id) -> bool {
    ui.label(label);
    let changed = VariableTextEdit::new(value, hints)
        .desired_width(FIELD_WIDTH)
        .id(id.with(label))
        .show(ui)
        .changed();
    ui.end_row();
    changed
}

fn secret_row(ui: &mut Ui, label: &str, value: &mut String, hints: &VariableHints, id: Id) -> bool {
    ui.label(label);
    let reveal_id = id.with(label).with(REVEAL_SALT);
    let revealed: bool = ui.data(|data| data.get_temp(reveal_id)).unwrap_or(false);
    let changed = ui
        .horizontal(|ui| {
            let edit = VariableTextEdit::new(value, hints)
                .desired_width(FIELD_WIDTH)
                .password(!revealed)
                .id(id.with(label))
                .show(ui);
            let toggle = if revealed { HIDE_LABEL } else { SHOW_LABEL };
            if ui.small_button(toggle).clicked() {
                ui.data_mut(|data| data.insert_temp(reveal_id, !revealed));
            }
            edit.changed()
        })
        .inner;
    ui.end_row();
    changed
}

fn location_row(ui: &mut Ui, in_query: &mut bool) -> bool {
    ui.label(ADD_TO_LABEL);
    let before = *in_query;
    ui.horizontal(|ui| {
        ui.selectable_value(in_query, false, HEADER_LABEL);
        ui.selectable_value(in_query, true, QUERY_LABEL);
    });
    ui.end_row();
    *in_query != before
}
