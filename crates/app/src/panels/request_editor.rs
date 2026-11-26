mod body;
mod explorer_row;
mod graphql;
mod inherited;
mod scripts;
mod settings;
mod tables;
mod url_bar;

use eframe::egui::{ScrollArea, Ui};
use requestor_core::settings::RequestSettings;

use crate::state::{Action, AppState, EditorSection, Inheritance, RequestTab, TabId};
use crate::widgets::auth_editor;
use crate::widgets::variable_text_edit::VariableHints;

const SECTION_SCROLL_SALT: &str = "request-editor-section";
const AUTH_SALT: &str = "request-auth";
const ATTENTION_MARK: &str = " !";

pub fn show(ui: &mut Ui, state: &mut AppState, tab: TabId, actions: &mut Vec<Action>) {
    let hints = VariableHints::new(
        state.variable_scope(tab),
        state.interface().variable_autocomplete,
    )
    .editable(state.variable_homes(tab));
    let code_panel_open = state.code_panel.is_some();
    let protected = state.sends_to_protected_environment();
    let global = state.workspace.settings.request.clone();
    let shows_inheritance = state.tab(tab).is_some_and(|open| {
        matches!(
            open.editor_section,
            EditorSection::Auth | EditorSection::Headers
        )
    });
    let inheritance = if shows_inheritance {
        state.inheritance(tab)
    } else {
        Inheritance::default()
    };
    let Some(open) = state.tab_mut(tab) else {
        return;
    };
    let context = EditorContext {
        hints: &hints,
        global: &global,
        inheritance: &inheritance,
        code_panel_open,
        protected,
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
    inheritance: &'a Inheritance,
    code_panel_open: bool,
    protected: bool,
}

fn show_tab(
    ui: &mut Ui,
    open: &mut RequestTab,
    context: &EditorContext,
    actions: &mut Vec<Action>,
) -> bool {
    let mut changed = url_bar::show(ui, open, context, actions);
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
        EditorSection::Scripts => open.draft.scripts.active_count(),
        EditorSection::Settings => open.draft.overrides.count(),
    };
    let label = match count {
        0 => section.label().to_string(),
        count => format!("{} ({count})", section.label()),
    };
    let attention = match section {
        EditorSection::Settings => open.draft.overrides.lowers_security(),
        EditorSection::Scripts => open.draft.scripts.has_problems(),
        _ => false,
    };
    if attention {
        return format!("{label}{ATTENTION_MARK}");
    }
    label
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
        EditorSection::Headers => tables::headers(
            ui,
            &mut open.draft,
            &mut open.header_bulk,
            hints,
            &context.inheritance.headers,
        ),
        EditorSection::Body => body::show(ui, open, hints, actions),
        EditorSection::Auth => {
            let changed = auth_editor::show(ui, &mut open.draft.auth, hints, AUTH_SALT);
            inherited::auth(ui, &open.draft.auth, context.inheritance.auth.as_ref());
            changed
        }
        EditorSection::Scripts => scripts::show(ui, open, hints),
        EditorSection::Settings => settings::show(ui, &mut open.draft.overrides, context.global),
    }
}
