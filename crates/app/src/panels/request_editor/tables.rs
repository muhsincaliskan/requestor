use eframe::egui::{Layout, Ui};
use requestor_core::collection::InheritedHeader;
use requestor_core::draft::{KeyValueRow, RequestDraft};

use crate::widgets::key_value_table::KeyValueTable;
use crate::widgets::variable_text_edit::VariableHints;

const QUERY_TITLE: &str = "Query parameters";
const PATH_TITLE: &str = "Path variables";
const HEADERS_TITLE: &str = "Headers";
const QUERY_SALT: &str = "query";
const PATH_SALT: &str = "path-variables";
const HEADERS_SALT: &str = "headers";
const HEADERS_BULK_SALT: &str = "headers-bulk";
const BULK_EDIT: &str = "Bulk edit";
const KEY_VALUE_EDIT: &str = "Key-value edit";
const BULK_ROWS: usize = 8;

pub fn params(ui: &mut Ui, draft: &mut RequestDraft, hints: &VariableHints) -> bool {
    ui.strong(QUERY_TITLE);
    let query_changed = KeyValueTable::new(&mut draft.query, hints, QUERY_SALT).show(ui);
    if query_changed {
        draft.query_edited();
    }
    if draft.path_variables.is_empty() {
        return query_changed;
    }
    ui.add_space(ui.spacing().item_spacing.y * 2.0);
    ui.strong(PATH_TITLE);
    let path_changed = KeyValueTable::new(&mut draft.path_variables, hints, PATH_SALT)
        .fixed_keys()
        .show(ui);
    query_changed || path_changed
}

pub fn headers(
    ui: &mut Ui,
    draft: &mut RequestDraft,
    bulk: &mut Option<String>,
    hints: &VariableHints,
    inherited: &[InheritedHeader],
) -> bool {
    headers_title(ui, draft, bulk);
    let changed = match bulk {
        Some(text) => bulk_headers(ui, draft, text, hints),
        None => KeyValueTable::new(&mut draft.headers, hints, HEADERS_SALT).show(ui),
    };
    super::inherited::headers(ui, draft, inherited);
    changed
}

fn headers_title(ui: &mut Ui, draft: &RequestDraft, bulk: &mut Option<String>) {
    ui.horizontal(|ui| {
        ui.strong(HEADERS_TITLE);
        ui.with_layout(Layout::right_to_left(ui.layout().vertical_align()), |ui| {
            let label = if bulk.is_some() {
                KEY_VALUE_EDIT
            } else {
                BULK_EDIT
            };
            if ui.link(label).clicked() {
                *bulk = match bulk {
                    Some(_) => None,
                    None => Some(KeyValueRow::bulk_text(&draft.headers)),
                };
            }
        });
    });
}

fn bulk_headers(
    ui: &mut Ui,
    draft: &mut RequestDraft,
    text: &mut String,
    hints: &VariableHints,
) -> bool {
    let changed = super::body::scrolling_editor(ui, text, hints, HEADERS_BULK_SALT, BULK_ROWS);
    if changed {
        draft.headers = KeyValueRow::rows_from_bulk(text, &draft.headers);
    }
    changed
}
