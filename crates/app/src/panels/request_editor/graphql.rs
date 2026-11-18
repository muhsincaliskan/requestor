use eframe::egui::{
    CursorIcon, Label, Response, RichText, ScrollArea, Sense, Spinner, TextEdit, TextStyle, Ui,
};
use requestor_core::draft::BodyDraft;
use requestor_core::graphql::{FieldFilter, GraphQlSchema, OperationKind, SchemaField};

use super::body::scrolling_editor;
use super::explorer_row::ExplorerRow;
use crate::state::{Action, SchemaLookup, SchemaView, TabId};
use crate::widgets::variable_text_edit::VariableHints;

const QUERY_SALT: &str = "body-graphql-query";
const VARIABLES_SALT: &str = "body-graphql-variables";
const EXPLORER_SALT: &str = "body-graphql-schema";
const QUERY_TITLE: &str = "Query";
const VARIABLES_TITLE: &str = "Variables (JSON)";
const SCHEMA_TITLE: &str = "Schema";
const FETCH_LABEL: &str = "Fetch schema";
const REFRESH_LABEL: &str = "Refresh schema";
const FETCHING_TEXT: &str = "Fetching the schema…";
const CANCEL_LABEL: &str = "Cancel";
const FETCH_HINT: &str =
    "Ask the server for its schema with the URL, headers and auth of this request";
const FILTER_HINT: &str = "Filter fields";
const NO_MATCH_TEXT: &str = "No field matches the filter.";
const USE_HINT: &str = "Click to replace the query with it. Filled variable values are kept.";
const TYPES_SUFFIX: &str = "types";
const SEPARATOR: &str = " · ";
const QUERY_ROWS: usize = 10;
const VARIABLES_ROWS: usize = 5;
const EXPLORER_COLUMNS: usize = 2;
const EXPLORER_MAX_HEIGHT: f32 = 260.0;

pub(super) fn show(
    ui: &mut Ui,
    body: &mut BodyDraft,
    schema: &mut SchemaView,
    tab: TabId,
    hints: &VariableHints,
    actions: &mut Vec<Action>,
) -> bool {
    schema_bar(ui, schema, tab, actions);
    let SchemaView { lookup, filter } = schema;
    let SchemaLookup::Ready(schema) = lookup else {
        return editors(ui, body, hints);
    };
    let mut changed = false;
    ui.columns(EXPLORER_COLUMNS, |columns| {
        changed |= editors(&mut columns[0], body, hints);
        changed |= explorer(&mut columns[1], body, schema, filter);
    });
    changed
}

fn editors(ui: &mut Ui, body: &mut BodyDraft, hints: &VariableHints) -> bool {
    ui.strong(QUERY_TITLE);
    let query = scrolling_editor(ui, &mut body.graphql_query, hints, QUERY_SALT, QUERY_ROWS);
    ui.strong(VARIABLES_TITLE);
    let variables = scrolling_editor(
        ui,
        &mut body.graphql_variables,
        hints,
        VARIABLES_SALT,
        VARIABLES_ROWS,
    );
    query | variables
}

fn schema_bar(ui: &mut Ui, schema: &SchemaView, tab: TabId, actions: &mut Vec<Action>) {
    ui.horizontal_wrapped(|ui| {
        if schema.is_fetching() {
            if ui.button(CANCEL_LABEL).clicked() {
                actions.push(Action::CancelSchema(tab));
            }
        } else {
            let label = match schema.lookup {
                SchemaLookup::Ready(_) => REFRESH_LABEL,
                _ => FETCH_LABEL,
            };
            if ui.button(label).on_hover_text(FETCH_HINT).clicked() {
                actions.push(Action::FetchSchema(tab));
            }
        }
        match &schema.lookup {
            SchemaLookup::Idle => {}
            SchemaLookup::Fetching(_) => {
                ui.add(Spinner::new());
                ui.weak(FETCHING_TEXT);
            }
            SchemaLookup::Ready(schema) => {
                ui.weak(summary(schema));
            }
            SchemaLookup::Failed(reason) => {
                ui.colored_label(ui.visuals().error_fg_color, reason.as_str());
            }
        }
    });
}

fn summary(schema: &GraphQlSchema) -> String {
    let mut parts = vec![format!("{} {TYPES_SUFFIX}", schema.type_count())];
    for kind in OperationKind::ALL {
        let count = schema.root_fields(kind).len();
        if count > 0 {
            parts.push(format!("{} {count}", kind.label()));
        }
    }
    parts.join(SEPARATOR)
}

fn explorer(
    ui: &mut Ui,
    body: &mut BodyDraft,
    schema: &GraphQlSchema,
    filter: &mut String,
) -> bool {
    ui.strong(SCHEMA_TITLE);
    ui.add(
        TextEdit::singleline(filter)
            .hint_text(FILTER_HINT)
            .desired_width(f32::INFINITY),
    );
    let rows = ExplorerRow::listed(schema, &FieldFilter::new(filter));
    if rows.is_empty() {
        ui.weak(NO_MATCH_TEXT);
        return false;
    }
    let row_height = ui.text_style_height(&TextStyle::Monospace) + ui.spacing().item_spacing.y;
    let mut chosen = None;
    ScrollArea::vertical()
        .id_salt(EXPLORER_SALT)
        .max_height(EXPLORER_MAX_HEIGHT)
        .auto_shrink([false, true])
        .show_rows(ui, row_height, rows.len(), |ui, shown| {
            for row in &rows[shown] {
                match row {
                    ExplorerRow::Heading(kind) => {
                        ui.weak(kind.label());
                    }
                    ExplorerRow::Field(kind, field) => {
                        if field_row(ui, field).clicked() {
                            chosen = Some((*kind, field.name.as_str()));
                        }
                    }
                }
            }
        });
    let operation = chosen.and_then(|(kind, name)| schema.operation(kind, name));
    match operation {
        Some(operation) => {
            body.use_graphql_operation(operation);
            true
        }
        None => false,
    }
}

fn field_row(ui: &mut Ui, field: &SchemaField) -> Response {
    let response = ui
        .add(
            Label::new(RichText::new(field.signature()).monospace())
                .truncate()
                .sense(Sense::click()),
        )
        .on_hover_cursor(CursorIcon::PointingHand);
    response.on_hover_ui(|ui| {
        if let Some(description) = &field.description {
            ui.label(description);
        }
        ui.weak(USE_HINT);
    })
}
