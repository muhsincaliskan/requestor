mod body;
mod changes;
mod checks;
mod cookie;
mod events;
mod lists;
mod readout;
mod socket;
mod status;

use std::time::Duration;

use eframe::egui::{Id, Label, RichText, Spinner, Ui};

use crate::response::ResponseView;
use crate::shortcuts;
use crate::state::{Action, AppState, Exchange, LiveStream, ResponseSection, TabId};

const IDLE_HINT: &str = "Send the request to see a response";
const SENDING_LABEL: &str = "Sending…";
const CANCEL_LABEL: &str = "Cancel";
const STOP_LABEL: &str = "Stop";
const RECEIVING_LABEL: &str = "Events received:";
const SEND_ELAPSED_ID: &str = "response-send-started";
const IDLE_TOP_SHARE: f32 = 0.35;
const SENDING_REPAINT: Duration = Duration::from_millis(100);

pub fn show(ui: &mut Ui, state: &mut AppState, tab: TabId, actions: &mut Vec<Action>) {
    let Some(open) = state.tab_mut(tab) else {
        return;
    };
    let section = open.response_section;
    let example_source = open.binding.is_some().then_some(tab);
    let mut chosen = section;
    match &mut open.exchange {
        Exchange::Idle => idle(ui),
        Exchange::Sending { job, live, .. } => sending(ui, tab, *job, live.as_deref(), actions),
        Exchange::Failed(reason) => failed(ui, reason),
        Exchange::Completed(view) => completed(ui, view, example_source, &mut chosen, actions),
        Exchange::Socket(session) => {
            socket::show(ui, tab, session, &mut open.message_draft, actions);
        }
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

fn sending(
    ui: &mut Ui,
    tab: TabId,
    job: u64,
    live: Option<&LiveStream>,
    actions: &mut Vec<Action>,
) {
    let now = ui.input(|input| input.time);
    let key = Id::new((SEND_ELAPSED_ID, tab, job));
    let first_seen = ui.data_mut(|data| *data.get_temp_mut_or_insert_with(key, || now));
    let elapsed = (now - first_seen).max(0.0);
    ui.ctx().request_repaint_after(SENDING_REPAINT);
    ui.horizontal(|ui| {
        ui.add(Spinner::new());
        let (status, stop) = match live {
            Some(stream) => (
                format!(
                    "{RECEIVING_LABEL} {} · {elapsed:.0} s",
                    stream.log().total()
                ),
                STOP_LABEL,
            ),
            None => (format!("{SENDING_LABEL} {elapsed:.1} s"), CANCEL_LABEL),
        };
        ui.label(status);
        if ui.button(stop).clicked() {
            actions.push(Action::CancelSend(tab));
        }
    });
    if let Some(stream) = live {
        ui.separator();
        events::list(ui, Some(stream.log()), actions);
    }
}

fn failed(ui: &mut Ui, reason: &str) {
    let message = RichText::new(reason).color(ui.visuals().error_fg_color);
    ui.add(Label::new(message).selectable(true));
}

fn completed(
    ui: &mut Ui,
    view: &mut ResponseView,
    example_source: Option<TabId>,
    section: &mut ResponseSection,
    actions: &mut Vec<Action>,
) {
    status::show(ui, view, example_source, actions);
    if section_label(*section, view).is_none() {
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
        ResponseSection::Changes => changes::show(ui, view),
        ResponseSection::Events => events::list(ui, view.events(), actions),
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
        ResponseSection::Changes => view.has_previous().then(|| label.to_string()),
        ResponseSection::Events => view
            .events()
            .map(|log| format!("{label} ({})", log.total())),
    }
}
