use eframe::egui::{ComboBox, ScrollArea, Ui};
use requestor_core::draft::{BodyDraft, BodyKind, RawLanguage};

use crate::widgets::key_value_table::KeyValueTable;
use crate::widgets::variable_text_edit::VariableTextEdit;

const LANGUAGE_SALT: &str = "body-language";
const URLENCODED_SALT: &str = "body-urlencoded";
const FORMDATA_SALT: &str = "body-formdata";
const SCROLL_SALT: &str = "scroll";
const FILE_SALT: &str = "body-file";
const RAW_SALT: &str = "body-raw";
const QUERY_SALT: &str = "body-graphql-query";
const VARIABLES_SALT: &str = "body-graphql-variables";
const FILE_HINT: &str = "Path of the file to send";
const BEAUTIFY_LABEL: &str = "Beautify";
const NONE_TEXT: &str = "This request has no body.";
const QUERY_TITLE: &str = "Query";
const VARIABLES_TITLE: &str = "Variables (JSON)";
const EDITOR_MAX_HEIGHT: f32 = 320.0;
const RAW_ROWS: usize = 14;
const QUERY_ROWS: usize = 10;
const VARIABLES_ROWS: usize = 5;

pub fn show(ui: &mut Ui, body: &mut BodyDraft, names: &[String]) -> bool {
    let mut changed = kind_selector(ui, body);
    ui.add_space(ui.spacing().item_spacing.y);
    changed |= match body.kind {
        BodyKind::None => none_text(ui),
        BodyKind::Raw => raw_editor(ui, body, names),
        BodyKind::Urlencoded => {
            KeyValueTable::new(&mut body.urlencoded, names, URLENCODED_SALT).show(ui)
        }
        BodyKind::FormData => KeyValueTable::new(&mut body.formdata, names, FORMDATA_SALT)
            .with_files()
            .show(ui),
        BodyKind::File => file_editor(ui, body, names),
        BodyKind::GraphQl => graphql_editor(ui, body, names),
    };
    changed
}

fn kind_selector(ui: &mut Ui, body: &mut BodyDraft) -> bool {
    let before = body.kind;
    ui.horizontal_wrapped(|ui| {
        for kind in BodyKind::ALL {
            ui.selectable_value(&mut body.kind, kind, kind.label());
        }
    });
    body.kind != before
}

fn none_text(ui: &mut Ui) -> bool {
    ui.weak(NONE_TEXT);
    false
}

fn raw_editor(ui: &mut Ui, body: &mut BodyDraft, names: &[String]) -> bool {
    let before = body.language;
    let beautified = ui
        .horizontal(|ui| {
            ComboBox::from_id_salt(LANGUAGE_SALT)
                .selected_text(body.language.label())
                .show_ui(ui, |ui| {
                    for language in RawLanguage::ALL {
                        ui.selectable_value(&mut body.language, language, language.label());
                    }
                });
            body.can_beautify() && ui.button(BEAUTIFY_LABEL).clicked() && body.beautify()
        })
        .inner;
    let language_changed = body.language != before;
    language_changed | beautified | scrolling_editor(ui, &mut body.raw, names, RAW_SALT, RAW_ROWS)
}

fn file_editor(ui: &mut Ui, body: &mut BodyDraft, names: &[String]) -> bool {
    VariableTextEdit::new(&mut body.file_path, names)
        .hint_text(FILE_HINT)
        .desired_width(f32::INFINITY)
        .id(ui.id().with(FILE_SALT))
        .show(ui)
        .changed()
}

fn graphql_editor(ui: &mut Ui, body: &mut BodyDraft, names: &[String]) -> bool {
    ui.strong(QUERY_TITLE);
    let query = scrolling_editor(ui, &mut body.graphql_query, names, QUERY_SALT, QUERY_ROWS);
    ui.strong(VARIABLES_TITLE);
    let variables = scrolling_editor(
        ui,
        &mut body.graphql_variables,
        names,
        VARIABLES_SALT,
        VARIABLES_ROWS,
    );
    query | variables
}

fn scrolling_editor(
    ui: &mut Ui,
    text: &mut String,
    names: &[String],
    salt: &str,
    rows: usize,
) -> bool {
    let id = ui.id().with(salt);
    ScrollArea::vertical()
        .id_salt(id.with(SCROLL_SALT))
        .max_height(EDITOR_MAX_HEIGHT)
        .show(ui, |ui| {
            VariableTextEdit::new(text, names)
                .multiline(rows)
                .desired_width(f32::INFINITY)
                .id(id)
                .show(ui)
                .changed()
        })
        .inner
}
