mod body;
mod checks;
mod explorer_row;
mod graphql;
mod settings;
mod tables;
mod url_bar;

use eframe::egui::{ScrollArea, Ui};
use requestor_core::settings::RequestSettings;

use crate::state::{Action, AppState, EditorSection, RequestTab, TabId};
use crate::widgets::auth_editor;
use crate::widgets::variable_text_edit::VariableHints;

const SECTION_SCROLL_SALT: &str = "request-editor-section";
const AUTH_SALT: &str = "request-auth";

pub fn show(ui: &mut Ui, state: &mut AppState, tab: TabId, actions: &mut Vec<Action>) {
    let hints = VariableHints::new(
        state.variable_scope(tab),
        state.interface().variable_autocomplete,
    );
    let code_panel_open = state.code_panel.is_some();
    let global = state.workspace.settings.request.clone();
    let Some(open) = state.tab_mut(tab) else {
        return;
    };
    let context = EditorContext {
        hints: &hints,
        global: &global,
        code_panel_open,
    };
    let changed = ui
        .push_id(tab, |ui| show_tab(ui, open, &context, actions))
        .inner;
    if changed {
        actions.push(Action::TabEdited(tab));
    }
}

struct EditorContext<'a> {
    hints: &'a VariableHints,
    global: &'a RequestSettings,
    code_panel_open: bool,
}

fn show_tab(
    ui: &mut Ui,
    open: &mut RequestTab,
    context: &EditorContext,
    actions: &mut Vec<Action>,
) -> bool {
    let mut changed = url_bar::show(ui, open, context.hints, context.code_panel_open, actions);
    ui.add_space(ui.spacing().item_spacing.y);
    section_selector(ui, open);
    ui.separator();
    ScrollArea::vertical()
        .id_salt(SECTION_SCROLL_SALT)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            changed |= show_section(ui, open, context, actions);
        });
    changed
}

fn section_selector(ui: &mut Ui, open: &mut RequestTab) {
    ui.horizontal(|ui| {
        for section in EditorSection::ALL {
            let label = section_label(open, section);
            ui.selectable_value(&mut open.editor_section, section, label);
        }
    });
}

fn section_label(open: &RequestTab, section: EditorSection) -> String {
    let count = match section {
        EditorSection::Params => open.draft.active_param_count(),
        EditorSection::Headers => open.draft.active_header_count(),
        EditorSection::Body => open.draft.body.active_count(),
        EditorSection::Auth => 0,
        EditorSection::Tests => open.draft.rules.active_count(),
        EditorSection::Settings => open.draft.overrides.count(),
    };
    match count {
        0 => section.label().to_string(),
        count => format!("{} ({count})", section.label()),
    }
}

fn show_section(
    ui: &mut Ui,
    open: &mut RequestTab,
    context: &EditorContext,
    actions: &mut Vec<Action>,
) -> bool {
    let hints = context.hints;
    match open.editor_section {
        EditorSection::Params => tables::params(ui, &mut open.draft, hints),
        EditorSection::Headers => tables::headers(ui, &mut open.draft, hints),
        EditorSection::Body => body::show(ui, open, hints, actions),
        EditorSection::Auth => auth_editor::show(ui, &mut open.draft.auth, hints, AUTH_SALT),
        EditorSection::Tests => checks::show(ui, &mut open.draft.rules),
        EditorSection::Settings => settings::show(ui, &mut open.draft.overrides, context.global),
    }
}
