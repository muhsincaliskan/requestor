use eframe::egui::{Button, Ui};
use requestor_core::collection::{Auth, Param};
use requestor_core::draft::AuthDraft;

use super::table::{delete_cell, edit_rows, enabled_cell, text_cell, RowEdit, VALUE_HINT};
use crate::widgets::auth_editor;
use crate::widgets::variable_text_edit::VariableHints;

const AUTH_TITLE: &str = "Authorization";
const AUTH_HINT: &str = "Requests inside that are set to Inherit from parent use this.";
const AUTH_SALT: &str = "shared-auth";
const HEADERS_TITLE: &str = "Headers";
const HEADERS_HINT: &str =
    "Sent with every request inside. A header with the same name on a folder or request wins.";
const HEADERS_TABLE_ID: &str = "shared-headers";
const HEADER_HINT: &str = "Header";
const ADD_HEADER_LABEL: &str = "+ Add header";
const HEADER_COLUMNS: usize = 4;

pub(super) fn auth_section(ui: &mut Ui, auth: &mut Option<Auth>, hints: &VariableHints) -> bool {
    ui.strong(AUTH_TITLE);
    ui.weak(AUTH_HINT);
    let mut draft = AuthDraft::from_auth(auth.as_ref());
    let changed = auth_editor::show(ui, &mut draft, hints, AUTH_SALT);
    if changed {
        *auth = draft.to_auth();
    }
    changed
}

pub(super) fn headers_section(ui: &mut Ui, headers: &mut Vec<Param>) -> bool {
    ui.strong(HEADERS_TITLE);
    ui.weak(HEADERS_HINT);
    let mut changed = edit_rows(ui, HEADERS_TABLE_ID, HEADER_COLUMNS, headers, header_row);
    if ui.add(Button::new(ADD_HEADER_LABEL).frame(false)).clicked() {
        headers.push(Param::text("", ""));
        changed = true;
    }
    changed
}

fn header_row(ui: &mut Ui, header: &mut Param) -> RowEdit {
    let mut changed = false;
    if let Some(enabled) = enabled_cell(ui, header.is_enabled()) {
        header.set_enabled(enabled);
        changed = true;
    }
    changed |= optional_text_cell(ui, &mut header.key, HEADER_HINT, false);
    let masked = header.has_secret_name();
    changed |= optional_text_cell(ui, &mut header.value, VALUE_HINT, masked);
    RowEdit::from_flags(changed, delete_cell(ui))
}

fn optional_text_cell(ui: &mut Ui, text: &mut Option<String>, hint: &str, masked: bool) -> bool {
    let mut edited = text.clone().unwrap_or_default();
    let changed = text_cell(ui, &mut edited, hint, masked);
    if changed {
        *text = Some(edited);
    }
    changed
}
