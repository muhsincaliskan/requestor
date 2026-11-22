use eframe::egui::text_edit::TextEditOutput;
use eframe::egui::{Response, RichText, Ui};
use requestor_core::variables::{TextSegments, VariablePreview};

use super::hints::VariableHints;

const SECRET_TEXT: &str = "Secret value hidden";
const UNDEFINED_TEXT: &str = "Not defined for this request";
const EMPTY_TEXT: &str = "Empty value";
const ELLIPSIS: &str = "…";
const MAX_PREVIEW_CHARS: usize = 200;

pub fn attach(hints: &VariableHints, text: &str, output: &TextEditOutput) -> Response {
    let response = output.response.response.clone();
    let Some(name) = placeholder_under_pointer(text, output) else {
        return response;
    };
    let preview = hints.preview(name);
    response.on_hover_ui_at_pointer(|ui| tooltip(ui, name, &preview))
}

pub fn placeholder_under_pointer<'a>(text: &'a str, output: &TextEditOutput) -> Option<&'a str> {
    let pointer = output.response.response.hover_pos()?;
    let index = output
        .galley
        .cursor_from_pos(pointer - output.galley_pos)
        .index
        .0;
    TextSegments::placeholder_at(text, index)
}

fn tooltip(ui: &mut Ui, name: &str, preview: &VariablePreview) {
    ui.strong(name);
    match preview {
        VariablePreview::Value(value) if value.is_empty() => {
            ui.weak(EMPTY_TEXT);
        }
        VariablePreview::Value(value) => {
            ui.label(RichText::new(shortened(value)).monospace());
        }
        VariablePreview::Secret => {
            ui.weak(SECRET_TEXT);
        }
        VariablePreview::Undefined => {
            ui.colored_label(ui.visuals().error_fg_color, UNDEFINED_TEXT);
        }
    }
}

fn shortened(value: &str) -> String {
    match value.char_indices().nth(MAX_PREVIEW_CHARS) {
        Some((cut, _)) => format!("{}{ELLIPSIS}", &value[..cut]),
        None => value.to_string(),
    }
}
