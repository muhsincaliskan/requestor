use eframe::egui::{ComboBox, RichText, TextEdit, Ui};
use requestor_core::checks::VariableTarget;
use requestor_core::draft::{CheckKind, CheckRow, ExtractKind, ExtractRow, RulesDraft};

use crate::widgets::icon_button;

const CUSTOM_SCRIPT_TEXT: &str =
    "This request has a test script written for Postman. Requestor keeps it unchanged but does not run it.";
const CHECKS_TITLE: &str = "Checks";
const CHECKS_EMPTY: &str =
    "Check the status, time, body, a header or a JSON value of every response.";
const EXTRACT_TITLE: &str = "Save values from the response";
const EXTRACT_EMPTY: &str = "Store a JSON value or a header in a variable for the next requests.";
const ADD_CHECK: &str = "Add check";
const ADD_VARIABLE: &str = "Add variable";
const DELETE_LABEL: &str = "×";
const DELETE_HINT: &str = "Delete row";
const INCOMPLETE_LABEL: &str = "!";
const INCOMPLETE_HINT: &str = "Incomplete. Fill in this row or it will not be saved.";
const VARIABLE_HINT: &str = "Variable name";
const FROM_LABEL: &str = "from";
const INTO_LABEL: &str = "in";
const KIND_WIDTH: f32 = 180.0;
const FIELD_WIDTH: f32 = 200.0;
const VARIABLE_WIDTH: f32 = 140.0;
const TARGET_WIDTH: f32 = 120.0;
const SECTION_GAP: f32 = 12.0;

pub fn show(ui: &mut Ui, rules: &mut RulesDraft) -> bool {
    if rules.is_custom_script() {
        ui.weak(CUSTOM_SCRIPT_TEXT);
        return false;
    }
    let checks = checks(ui, &mut rules.checks);
    ui.add_space(SECTION_GAP);
    checks | extractions(ui, &mut rules.extractions)
}

fn checks(ui: &mut Ui, rows: &mut Vec<CheckRow>) -> bool {
    ui.strong(CHECKS_TITLE);
    if rows.is_empty() {
        ui.weak(CHECKS_EMPTY);
    }
    let mut changed = false;
    let mut removed = None;
    for (index, row) in rows.iter_mut().enumerate() {
        ui.push_id(("check", index), |ui| {
            ui.horizontal(|ui| {
                changed |= check_row(ui, row);
                if row_end(ui, row.is_incomplete()) {
                    removed = Some(index);
                }
            });
        });
    }
    changed | edit_list(ui, rows, removed, ADD_CHECK)
}

fn check_row(ui: &mut Ui, row: &mut CheckRow) -> bool {
    let before = row.kind;
    ComboBox::from_id_salt("kind")
        .width(KIND_WIDTH)
        .selected_text(row.kind.label())
        .show_ui(ui, |ui| {
            for kind in CheckKind::ALL {
                ui.selectable_value(&mut row.kind, kind, kind.label());
            }
        });
    let mut changed = row.kind != before;
    if let Some(hint) = row.kind.subject_hint() {
        changed |= field(ui, &mut row.subject, hint, FIELD_WIDTH);
    }
    changed | field(ui, &mut row.expected, row.kind.expected_hint(), FIELD_WIDTH)
}

fn extractions(ui: &mut Ui, rows: &mut Vec<ExtractRow>) -> bool {
    ui.strong(EXTRACT_TITLE);
    if rows.is_empty() {
        ui.weak(EXTRACT_EMPTY);
    }
    let mut changed = false;
    let mut removed = None;
    for (index, row) in rows.iter_mut().enumerate() {
        ui.push_id(("extract", index), |ui| {
            ui.horizontal(|ui| {
                changed |= extract_row(ui, row);
                if row_end(ui, row.is_incomplete()) {
                    removed = Some(index);
                }
            });
        });
    }
    changed | edit_list(ui, rows, removed, ADD_VARIABLE)
}

fn extract_row(ui: &mut Ui, row: &mut ExtractRow) -> bool {
    let (kind, target) = (row.kind, row.target);
    let mut changed = field(ui, &mut row.variable, VARIABLE_HINT, VARIABLE_WIDTH);
    ui.label(FROM_LABEL);
    ComboBox::from_id_salt("kind")
        .selected_text(row.kind.label())
        .show_ui(ui, |ui| {
            for option in ExtractKind::ALL {
                ui.selectable_value(&mut row.kind, option, option.label());
            }
        });
    changed |= field(ui, &mut row.source, row.kind.source_hint(), FIELD_WIDTH);
    ui.label(INTO_LABEL);
    ComboBox::from_id_salt("target")
        .width(TARGET_WIDTH)
        .selected_text(row.target.label())
        .show_ui(ui, |ui| {
            for option in VariableTarget::ALL {
                ui.selectable_value(&mut row.target, option, option.label());
            }
        });
    changed | (row.kind != kind) | (row.target != target)
}

fn field(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> bool {
    ui.add(
        TextEdit::singleline(text)
            .hint_text(hint)
            .desired_width(width),
    )
    .changed()
}

fn row_end(ui: &mut Ui, incomplete: bool) -> bool {
    let deleted = icon_button(ui, DELETE_LABEL, DELETE_HINT).clicked();
    if incomplete {
        let mark = RichText::new(INCOMPLETE_LABEL).color(ui.visuals().warn_fg_color);
        ui.label(mark).on_hover_text(INCOMPLETE_HINT);
    }
    deleted
}

fn edit_list<T: Default>(
    ui: &mut Ui,
    rows: &mut Vec<T>,
    removed: Option<usize>,
    add: &str,
) -> bool {
    if let Some(index) = removed {
        rows.remove(index);
        return true;
    }
    let added = ui.button(add).clicked();
    if added {
        rows.push(T::default());
    }
    added
}
