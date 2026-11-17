mod body;
mod checks;
mod cookie;
mod lists;
mod readout;
mod status;

use std::time::Duration;

use eframe::egui::{Id, Label, RichText, Spinner, Ui};

use crate::response::ResponseView;
use crate::shortcuts;
use crate::state::{Action, AppState, Exchange, ResponseSection, TabId};

const IDLE_HINT: &str = "Send the request to see a response";
const SENDING_LABEL: &str = "Sending…";
const CANCEL_LABEL: &str = "Cancel";
const SEND_ELAPSED_ID: &str = "response-send-started";
const IDLE_TOP_SHARE: f32 = 0.35;
const SENDING_REPAINT: Duration = Duration::from_millis(100);

pub fn show(ui: &mut Ui, state: &mut AppState, tab: TabId, actions: &mut Vec<Action>) {
    let Some(open) = state.tab_mut(tab) else {
        return;
    };
    let section = open.response_section;
    let mut chosen = section;
    match &mut open.exchange {
        Exchange::Idle => idle(ui),
        Exchange::Sending { job, .. } => sending(ui, tab, *job, actions),
        Exchange::Failed(reason) => failed(ui, reason),
        Exchange::Completed(view) => completed(ui, view, &mut chosen, actions),
    }
    open.response_section = chosen;
}

fn idle(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * IDLE_TOP_SHARE);
        ui.weak(IDLE_HINT);
        if let Some(send) = shortcuts::ALL.first() {
            ui.weak(send.keys_text(ui.ctx()));
        }
    });
}

fn sending(ui: &mut Ui, tab: TabId, job: u64, actions: &mut Vec<Action>) {
    let now = ui.input(|input| input.time);
    let key = Id::new((SEND_ELAPSED_ID, tab, job));
    let first_seen = ui.data_mut(|data| *data.get_temp_mut_or_insert_with(key, || now));
    ui.ctx().request_repaint_after(SENDING_REPAINT);
    ui.horizontal(|ui| {
        ui.add(Spinner::new());
        ui.label(format!(
            "{SENDING_LABEL} {:.1} s",
            (now - first_seen).max(0.0)
        ));
        if ui.button(CANCEL_LABEL).clicked() {
            actions.push(Action::CancelSend(tab));
        }
    });
}

fn failed(ui: &mut Ui, reason: &str) {
    let message = RichText::new(reason).color(ui.visuals().error_fg_color);
    ui.add(Label::new(message).selectable(true));
}

fn completed(
    ui: &mut Ui,
    view: &mut ResponseView,
    section: &mut ResponseSection,
    actions: &mut Vec<Action>,
) {
    status::show(ui, view);
    if view.report.is_none() && *section == ResponseSection::Tests {
        *section = ResponseSection::Body;
    }
    ui.horizontal(|ui| {
        for candidate in ResponseSection::ALL {
            if let Some(label) = section_text(ui, candidate, view) {
                ui.selectable_value(section, candidate, label);
            }
        }
    });
    ui.separator();
    match *section {
        ResponseSection::Body => body::show(ui, view, actions),
        ResponseSection::Headers => lists::headers(ui, view, actions),
        ResponseSection::Cookies => lists::cookies(ui, view, actions),
        ResponseSection::Tests => checks::show(ui, view),
    }
}

fn section_text(ui: &Ui, section: ResponseSection, view: &ResponseView) -> Option<RichText> {
    let text = RichText::new(section_label(section, view)?);
    let failed = section == ResponseSection::Tests && view.has_failed_checks();
    Some(if failed {
        text.color(ui.visuals().error_fg_color)
    } else {
        text
    })
}

fn section_label(section: ResponseSection, view: &ResponseView) -> Option<String> {
    let label = section.label();
    match section {
        ResponseSection::Body => Some(label.to_string()),
        ResponseSection::Headers => Some(format!("{label} ({})", view.headers.len())),
        ResponseSection::Cookies => Some(format!("{label} ({})", view.cookies().count())),
        ResponseSection::Tests => view.report.as_ref().map(|report| {
            format!(
                "{label} ({}/{})",
                report.passed_count(),
                report.checks.len()
            )
        }),
    }
}
