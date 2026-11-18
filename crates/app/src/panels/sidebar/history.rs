use eframe::egui::{Id, Label, RichText, Sense, Ui};
use requestor_core::history::HistoryEntry;
use requestor_core::response::StatusClass;

use super::menus;
use super::tree::branch;
use crate::state::{Action, AppState};
use crate::widgets::{aligned_request_title, list_row, row_label, status_color};

const SECTION_ID: &str = "sidebar-history";
const TITLE: &str = "History";
const NO_RESPONSE_TEXT: &str = "No response";
const BODY_LEFT_OUT_TEXT: &str = "The body was too large to keep in history";
const HINT_SEPARATOR: &str = " · ";

pub(super) fn show(ui: &mut Ui, state: &AppState, filter: &str, actions: &mut Vec<Action>) {
    let history = &state.workspace.history;
    let shown = |entry: &&HistoryEntry| entry.matches_filter(filter);
    let count = if filter.is_empty() {
        history.len()
    } else {
        history.entries().iter().filter(shown).count()
    };
    if count == 0 {
        return;
    }
    let response = branch(
        ui,
        Id::new(SECTION_ID),
        !filter.is_empty(),
        |ui| {
            let title = RichText::new(format!("{TITLE} ({count})")).strong();
            ui.add(Label::new(title).sense(Sense::click()))
        },
        |ui| {
            for entry in history.entries().iter().filter(shown) {
                row(ui, entry, actions);
            }
        },
    );
    menus::history(&response, actions);
}

fn row(ui: &mut Ui, entry: &HistoryEntry, actions: &mut Vec<Action>) {
    let method = entry.method();
    let url = entry.url_without_scheme();
    let shown = if url.is_empty() { &entry.name } else { &url };
    let title = aligned_request_title(ui, Some(method.as_str()), shown);
    let outcome = outcome_text(ui, entry);
    let response = ui
        .add(list_row(ui, false, (row_label(title), outcome)))
        .on_hover_ui(|ui| {
            ui.label(hint(entry));
        });
    if response.clicked() {
        actions.push(Action::OpenHistoryEntry(Box::new(entry.clone())));
    }
}

fn outcome_text(ui: &Ui, entry: &HistoryEntry) -> RichText {
    match entry.status {
        Some(status) => {
            RichText::new(status.to_string()).color(status_color(ui, StatusClass::of(status)))
        }
        None => RichText::new(NO_RESPONSE_TEXT).color(ui.visuals().error_fg_color),
    }
    .small()
}

fn hint(entry: &HistoryEntry) -> String {
    let mut lines = vec![
        entry.url_text(),
        format!("{}{HINT_SEPARATOR}{}", entry.name, entry.age()),
    ];
    if entry.body_left_out {
        lines.push(BODY_LEFT_OUT_TEXT.to_string());
    }
    lines.join("\n")
}
