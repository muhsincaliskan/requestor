use eframe::egui::{TextEdit, Ui};
use requestor_core::draft::ScriptDraft;

use super::problems;
use crate::state::{ConversionFailure, ScriptPane};

const EDITOR_ROWS: usize = 14;
const NOTE: &str = "Postman JavaScript. Requestor never runs it. Convert it to a script to run it.";
const PENDING: &str = "Not converted yet. Changes here are not saved until you convert them.";
const CONVERT_LABEL: &str = "← Convert to script";
const COPY_LABEL: &str = "Copy";
const FAILED_TITLE: &str = "These lines could not be converted:";
const SCRIPT_ERRORS_TITLE: &str = "Fix the script first:";

pub fn show(ui: &mut Ui, pane: &mut ScriptPane, draft: &mut ScriptDraft) -> bool {
    ui.weak(NOTE);
    if draft.javascript_is_pending() {
        ui.colored_label(ui.visuals().warn_fg_color, PENDING);
    }
    ui.add(
        TextEdit::multiline(&mut draft.javascript)
            .code_editor()
            .desired_rows(EDITOR_ROWS)
            .desired_width(f32::INFINITY),
    );
    let mut changed = false;
    ui.horizontal(|ui| {
        if ui.button(CONVERT_LABEL).clicked() {
            changed = pane.convert_to_script(draft);
        }
        if ui.button(COPY_LABEL).clicked() {
            ui.ctx().copy_text(draft.javascript.clone());
        }
    });
    match &pane.failure {
        Some(ConversionFailure::JavaScript(unconverted)) => {
            ui.colored_label(ui.visuals().error_fg_color, FAILED_TITLE);
            problems::unconverted_lines(ui, unconverted);
        }
        Some(ConversionFailure::Script(errors)) => {
            ui.colored_label(ui.visuals().error_fg_color, SCRIPT_ERRORS_TITLE);
            problems::script_errors(ui, errors);
        }
        None => {}
    }
    changed
}
