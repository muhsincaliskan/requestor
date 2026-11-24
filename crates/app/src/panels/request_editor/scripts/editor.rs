use eframe::egui::{Context, Id, TextEdit, Ui};
use requestor_core::draft::ScriptDraft;
use requestor_core::script::{Completion, Phase};

use super::problems;
use crate::state::{ConversionFailure, ScriptPane};
use crate::widgets::variable_text_edit::{VariableHints, VariableTextEdit};

const EDITOR_SALT: &str = "script-editor";
const EDITOR_ROWS: usize = 14;
const CONVERT_LABEL: &str = "Convert to JavaScript →";
const SUGGESTIONS_LABEL: &str = "Next:";
const MAX_SUGGESTIONS: usize = 8;
const ERRORS_NOTE: &str = "Fix these lines. Until then the last saved script runs.";
const UNCONVERTED_TITLE: &str = "Couldn't convert this Postman script, so it does not run.";
const UNCONVERTED_NOTE: &str =
    "Requestor keeps the JavaScript unchanged. Write a script here to replace it, or fix the lines in the JavaScript view.";
const LINE_BREAK: char = '\n';

pub fn show(
    ui: &mut Ui,
    pane: &mut ScriptPane,
    draft: &mut ScriptDraft,
    hints: &VariableHints,
) -> bool {
    if let Some(unconverted) = draft.unconverted() {
        ui.colored_label(ui.visuals().warn_fg_color, UNCONVERTED_TITLE);
        problems::unconverted_lines(ui, unconverted);
        ui.weak(UNCONVERTED_NOTE);
        ui.add_space(ui.spacing().item_spacing.y);
    }
    let id = Id::new((EDITOR_SALT, draft.phase() == Phase::BeforeRequest));
    let mut changed = VariableTextEdit::new(&mut draft.text, hints)
        .multiline(EDITOR_ROWS)
        .desired_width(f32::INFINITY)
        .id(id)
        .show(ui)
        .changed();
    changed |= suggestions(ui, id, draft, hints.names());
    if let Err(errors) = draft.parsed() {
        problems::script_errors(ui, &errors);
        ui.weak(ERRORS_NOTE);
    }
    if ui.button(CONVERT_LABEL).clicked() {
        pane.convert_to_javascript(draft);
    }
    if let Some(ConversionFailure::JavaScript(unconverted)) = &pane.failure {
        problems::unconverted_lines(ui, unconverted);
    }
    changed
}

fn suggestions(ui: &mut Ui, id: Id, draft: &mut ScriptDraft, variables: &[String]) -> bool {
    let Some(cursor) = cursor(ui.ctx(), id) else {
        return false;
    };
    let line = line_before(&draft.text, cursor);
    let completion = Completion {
        phase: draft.phase(),
        variables,
    };
    let candidates = completion.suggestions(&line);
    if candidates.is_empty() {
        return false;
    }
    let mut chosen = None;
    ui.horizontal_wrapped(|ui| {
        ui.weak(SUGGESTIONS_LABEL);
        for candidate in candidates.iter().take(MAX_SUGGESTIONS) {
            if ui.small_button(candidate).clicked() {
                chosen = Some(candidate.clone());
            }
        }
    });
    let Some(candidate) = chosen else {
        return false;
    };
    let insertion = Completion::insertion(&line, &candidate);
    insert(ui.ctx(), id, &mut draft.text, cursor, &insertion);
    true
}

fn cursor(ctx: &Context, id: Id) -> Option<usize> {
    let state = TextEdit::load_state(ctx, id)?;
    Some(state.cursor.char_range()?.primary.index.into())
}

fn line_before(text: &str, cursor: usize) -> String {
    let before: String = text.chars().take(cursor).collect();
    let start = before.rfind(LINE_BREAK).map_or(0, |index| index + 1);
    before[start..].to_string()
}

fn insert(ctx: &Context, id: Id, text: &mut String, cursor: usize, insertion: &str) {
    let byte = text
        .char_indices()
        .nth(cursor)
        .map_or(text.len(), |(index, _)| index);
    text.insert_str(byte, insertion);
    let mut state = TextEdit::load_state(ctx, id).unwrap_or_default();
    let moved = eframe::egui::text::CCursor::new(cursor + insertion.chars().count());
    state
        .cursor
        .set_char_range(Some(eframe::egui::text::CCursorRange::one(moved)));
    TextEdit::store_state(ctx, id, state);
    ctx.memory_mut(|memory| memory.request_focus(id));
}
