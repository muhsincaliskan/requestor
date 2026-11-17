mod body;
mod tables;
mod url_bar;

use eframe::egui::{ScrollArea, Ui};

use crate::state::{Action, AppState, EditorSection, RequestTab, TabId};
use crate::widgets::auth_editor;

const SECTION_SCROLL_SALT: &str = "request-editor-section";
const AUTH_SALT: &str = "request-auth";

pub fn show(ui: &mut Ui, state: &mut AppState, tab: TabId, actions: &mut Vec<Action>) {
    let names = match state.interface().variable_autocomplete {
        true => state.variable_names(tab),
        false => Vec::new(),
    };
    let Some(open) = state.tab_mut(tab) else {
        return;
    };
    let changed = ui
        .push_id(tab, |ui| show_tab(ui, open, &names, actions))
        .inner;
    if changed {
        actions.push(Action::TabEdited(tab));
    }
}

fn show_tab(
    ui: &mut Ui,
    open: &mut RequestTab,
    names: &[String],
    actions: &mut Vec<Action>,
) -> bool {
    let mut changed = url_bar::show(ui, open, names, actions);
    ui.add_space(ui.spacing().item_spacing.y);
    section_selector(ui, open);
    ui.separator();
    ScrollArea::vertical()
        .id_salt(SECTION_SCROLL_SALT)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            changed |= show_section(ui, open, names);
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
    };
    match count {
        0 => section.label().to_string(),
        count => format!("{} ({count})", section.label()),
    }
}

fn show_section(ui: &mut Ui, open: &mut RequestTab, names: &[String]) -> bool {
    match open.editor_section {
        EditorSection::Params => tables::params(ui, &mut open.draft, names),
        EditorSection::Headers => tables::headers(ui, &mut open.draft, names),
        EditorSection::Body => body::show(ui, &mut open.draft.body, names),
        EditorSection::Auth => auth_editor::show(ui, &mut open.draft.auth, names, AUTH_SALT),
    }
}
