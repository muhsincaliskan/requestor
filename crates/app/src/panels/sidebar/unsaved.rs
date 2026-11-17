use eframe::egui::{Id, Label, RichText, Sense, Ui};

use super::tree::branch;
use super::{compact_rows, row, row_label, SECTION_GAP};
use crate::state::{Action, AppState, RequestTab};
use crate::widgets::aligned_request_title;

const NODE_ID: &str = "sidebar-unsaved";
const TITLE: &str = "Unsaved";
const OPEN_HINT: &str = "Open this tab";

pub(super) fn show(ui: &mut Ui, state: &AppState, actions: &mut Vec<Action>) {
    let tabs = state.unsaved_list();
    if tabs.is_empty() {
        return;
    }
    ui.scope(|ui| {
        compact_rows(ui);
        group(ui, state, &tabs, actions);
    });
    ui.add_space(SECTION_GAP);
}

fn group(ui: &mut Ui, state: &AppState, tabs: &[&RequestTab], actions: &mut Vec<Action>) {
    branch(
        ui,
        Id::new(NODE_ID),
        false,
        |ui| {
            let title = RichText::new(format!("{TITLE} ({})", tabs.len())).strong();
            ui.add(Label::new(title).sense(Sense::click()))
        },
        |ui| {
            for tab in tabs {
                let active = state.active_tab == Some(tab.id);
                let title = aligned_request_title(ui, Some(&tab.draft.method), &tab.draft.name);
                let response = ui
                    .add(row(ui, active, row_label(title)))
                    .on_hover_text(OPEN_HINT);
                if response.clicked() {
                    actions.push(Action::ActivateTab(tab.id));
                }
            }
        },
    );
}
