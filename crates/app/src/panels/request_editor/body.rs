use eframe::egui::{ComboBox, ScrollArea, Ui};
use requestor_core::draft::{BodyDraft, BodyKind, RawLanguage, RequestDraft};
use requestor_core::soap::{SoapVersion, SOAP_ACTION_HEADER};

use super::graphql;
use crate::state::{Action, RequestTab};
use crate::widgets::key_value_table::KeyValueTable;
use crate::widgets::variable_text_edit::{VariableHints, VariableTextEdit};

const LANGUAGE_SALT: &str = "body-language";
const URLENCODED_SALT: &str = "body-urlencoded";
const FORMDATA_SALT: &str = "body-formdata";
const SCROLL_SALT: &str = "scroll";
const FILE_SALT: &str = "body-file";
const RAW_SALT: &str = "body-raw";
const SOAP_ACTION_SALT: &str = "body-soap-action";
const FILE_HINT: &str = "Path of the file to send";
const SOAP_ACTION_HINT: &str = "Optional, for example urn:GetUser";
const SOAP_ACTION_WIDTH: f32 = 320.0;
const BEAUTIFY_LABEL: &str = "Beautify";
const NONE_TEXT: &str = "This request has no body.";
const EDITOR_MAX_HEIGHT: f32 = 320.0;
const RAW_ROWS: usize = 14;

pub fn show(
    ui: &mut Ui,
    open: &mut RequestTab,
    hints: &VariableHints,
    actions: &mut Vec<Action>,
) -> bool {
    let draft = &mut open.draft;
    let changed = kind_selector(ui, &mut draft.body);
    ui.add_space(ui.spacing().item_spacing.y);
    changed
        | match draft.body.kind {
            BodyKind::None => none_text(ui),
            BodyKind::Raw => raw_editor(ui, &mut draft.body, hints),
            BodyKind::Urlencoded => {
                KeyValueTable::new(&mut draft.body.urlencoded, hints, URLENCODED_SALT).show(ui)
            }
            BodyKind::FormData => {
                KeyValueTable::new(&mut draft.body.formdata, hints, FORMDATA_SALT)
                    .with_files()
                    .show(ui)
            }
            BodyKind::File => file_editor(ui, &mut draft.body, hints),
            BodyKind::GraphQl => graphql::show(
                ui,
                &mut draft.body,
                &mut open.schema,
                open.id,
                hints,
                actions,
            ),
            BodyKind::Soap => soap_editor(ui, draft, hints),
        }
}

fn kind_selector(ui: &mut Ui, body: &mut BodyDraft) -> bool {
    let mut chosen = body.kind;
    ui.horizontal_wrapped(|ui| {
        for kind in BodyKind::ALL {
            ui.selectable_value(&mut chosen, kind, kind.label());
        }
    });
    let changed = chosen != body.kind;
    if changed {
        body.switch_to(chosen);
    }
    changed
}

fn none_text(ui: &mut Ui) -> bool {
    ui.weak(NONE_TEXT);
    false
}

fn raw_editor(ui: &mut Ui, body: &mut BodyDraft, hints: &VariableHints) -> bool {
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
    language_changed | beautified | scrolling_editor(ui, &mut body.raw, hints, RAW_SALT, RAW_ROWS)
}

fn soap_editor(ui: &mut Ui, draft: &mut RequestDraft, hints: &VariableHints) -> bool {
    let body = &mut draft.body;
    let mut changed = ui
        .horizontal(|ui| {
            let current = body.soap_version();
            let mut converted = false;
            for version in SoapVersion::ALL {
                if ui
                    .selectable_label(current == Some(version), version.label())
                    .clicked()
                {
                    converted |= body.use_soap_version(version);
                }
            }
            let beautified = ui.button(BEAUTIFY_LABEL).clicked() && body.beautify();
            converted | beautified
        })
        .inner;
    changed |= soap_action_field(ui, draft, hints);
    changed | scrolling_editor(ui, &mut draft.body.raw, hints, RAW_SALT, RAW_ROWS)
}

fn soap_action_field(ui: &mut Ui, draft: &mut RequestDraft, hints: &VariableHints) -> bool {
    let mut action = draft.soap_action().to_string();
    let changed = ui
        .horizontal(|ui| {
            ui.label(SOAP_ACTION_HEADER);
            VariableTextEdit::new(&mut action, hints)
                .hint_text(SOAP_ACTION_HINT)
                .desired_width(SOAP_ACTION_WIDTH)
                .id(ui.id().with(SOAP_ACTION_SALT))
                .show(ui)
                .changed()
        })
        .inner;
    if changed {
        draft.set_soap_action(&action);
    }
    changed
}

fn file_editor(ui: &mut Ui, body: &mut BodyDraft, hints: &VariableHints) -> bool {
    VariableTextEdit::new(&mut body.file_path, hints)
        .hint_text(FILE_HINT)
        .desired_width(f32::INFINITY)
        .id(ui.id().with(FILE_SALT))
        .show(ui)
        .changed()
}

pub(super) fn scrolling_editor(
    ui: &mut Ui,
    text: &mut String,
    hints: &VariableHints,
    salt: &str,
    rows: usize,
) -> bool {
    let id = ui.id().with(salt);
    ScrollArea::vertical()
        .id_salt(id.with(SCROLL_SALT))
        .max_height(EDITOR_MAX_HEIGHT)
        .show(ui, |ui| {
            VariableTextEdit::new(text, hints)
                .multiline(rows)
                .desired_width(f32::INFINITY)
                .id(id)
                .show(ui)
                .changed()
        })
        .inner
}
