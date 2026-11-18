use eframe::egui::Ui;
use requestor_core::collection::InheritedHeader;
use requestor_core::draft::RequestDraft;

use crate::widgets::key_value_table::KeyValueTable;
use crate::widgets::variable_text_edit::VariableHints;

const QUERY_TITLE: &str = "Query parameters";
const PATH_TITLE: &str = "Path variables";
const HEADERS_TITLE: &str = "Headers";
const QUERY_SALT: &str = "query";
const PATH_SALT: &str = "path-variables";
const HEADERS_SALT: &str = "headers";

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
    hints: &VariableHints,
    inherited: &[InheritedHeader],
) -> bool {
    ui.strong(HEADERS_TITLE);
    let changed = KeyValueTable::new(&mut draft.headers, hints, HEADERS_SALT).show(ui);
    super::inherited::headers(ui, draft, inherited);
    changed
}
