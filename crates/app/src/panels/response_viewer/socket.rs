use eframe::egui::text::LayoutJob;
use eframe::egui::{
    vec2, Align, Color32, FontId, Label, Layout, ScrollArea, Sense, Spinner, TextEdit, TextFormat,
    TextStyle, Ui,
};
use requestor_core::websocket::{Direction, SocketMessage, SocketPayload};

use super::body::digits;
use super::readout::{ByteSize, ElapsedText};
use super::status::badge;
use crate::shortcuts;
use crate::state::{Action, SocketPhase, SocketSession, TabId};
use crate::widgets::{action_button, success_color};

const SCROLL_ID: &str = "response-socket-messages";
const CONNECTING_LABEL: &str = "Connecting…";
const OPEN_LABEL: &str = "Connected";
const CLOSING_LABEL: &str = "Closing…";
const CLOSED_LABEL: &str = "Closed";
const MESSAGES_SUFFIX: &str = "messages";
const SEPARATOR: &str = "·";
const NO_MESSAGES_TEXT: &str = "No messages yet.";
const MESSAGE_HINT: &str = "Message to send";
const SEND_MESSAGE_LABEL: &str = "Send";
const SENT_MARK: &str = "↑";
const RECEIVED_MARK: &str = "↓";
const BINARY_PREFIX: &str = "Binary message,";
const COPY_HINT: &str = "Click to copy the message";
const LINE_BREAK_MARK: &str = " ⏎ ";
const COLUMN_GAP: &str = "  ";
const TIME_PREFIX: &str = "+";
const TIME_WIDTH: usize = 9;
const MAX_SHOWN_CHARS: usize = 400;
const COMPOSER_ROWS: usize = 3;
const SEND_BUTTON_WIDTH: f32 = 72.0;

pub(super) fn show(
    ui: &mut Ui,
    tab: TabId,
    session: &SocketSession,
    draft: &mut String,
    actions: &mut Vec<Action>,
) {
    header(ui, session);
    ui.separator();
    ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
        if session.is_open() {
            composer(ui, tab, draft, actions);
            ui.separator();
        }
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            messages(ui, session, actions);
        });
    });
}

fn header(ui: &mut Ui, session: &SocketSession) {
    ui.horizontal_wrapped(|ui| {
        let visuals = ui.visuals();
        let (label, color) = match session.phase() {
            SocketPhase::Connecting => (CONNECTING_LABEL.to_string(), visuals.text_color()),
            SocketPhase::Open => (OPEN_LABEL.to_string(), success_color(ui)),
            SocketPhase::Closing => (CLOSING_LABEL.to_string(), visuals.text_color()),
            SocketPhase::Closed(reason) => {
                (closed_text(reason.as_deref()), visuals.weak_text_color())
            }
            SocketPhase::Failed(reason) => (reason.clone(), visuals.error_fg_color),
        };
        if session.is_live() && !session.is_open() {
            ui.add(Spinner::new());
        }
        badge(ui, &label, color);
        if let Some(head) = session.head() {
            ui.weak(SEPARATOR);
            ui.label(ElapsedText(head.elapsed).to_string());
        }
        ui.weak(SEPARATOR);
        ui.label(format!("{} {MESSAGES_SUFFIX}", session.log().total()));
    });
}

fn closed_text(reason: Option<&str>) -> String {
    match reason {
        Some(reason) => format!("{CLOSED_LABEL} ({reason})"),
        None => CLOSED_LABEL.to_string(),
    }
}

fn composer(ui: &mut Ui, tab: TabId, draft: &mut String, actions: &mut Vec<Action>) {
    let row = ui.text_style_height(&TextStyle::Monospace);
    let height = COMPOSER_ROWS as f32 * row + 2.0 * ui.spacing().button_padding.y;
    let size = vec2(ui.available_width(), height);
    ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
        let width = ui.available_width() - SEND_BUTTON_WIDTH;
        ui.add(
            TextEdit::multiline(draft)
                .code_editor()
                .desired_rows(COMPOSER_ROWS)
                .desired_width(width)
                .hint_text(hint(ui)),
        );
        if action_button::primary(ui, SEND_MESSAGE_LABEL).clicked() {
            actions.push(Action::SendSocketMessage(tab));
        }
    });
}

fn hint(ui: &Ui) -> String {
    match shortcuts::ALL.first() {
        Some(send) => format!("{MESSAGE_HINT} ({})", send.keys_text(ui.ctx())),
        None => MESSAGE_HINT.to_string(),
    }
}

fn messages(ui: &mut Ui, session: &SocketSession, actions: &mut Vec<Action>) {
    let log = session.log();
    if log.kept() == 0 {
        ui.weak(NO_MESSAGES_TEXT);
        return;
    }
    let row_height = ui.text_style_height(&TextStyle::Monospace);
    let width = digits(log.total());
    ScrollArea::vertical()
        .id_salt(SCROLL_ID)
        .auto_shrink(false)
        .stick_to_bottom(true)
        .show_rows(ui, row_height, log.kept(), |ui, rows| {
            for index in rows {
                if let Some(message) = log.get(index) {
                    row(ui, log.number_of(index), width, message, actions);
                }
            }
        });
}

fn row(
    ui: &mut Ui,
    number: usize,
    width: usize,
    message: &SocketMessage,
    actions: &mut Vec<Action>,
) {
    let font = TextStyle::Monospace.resolve(ui.style());
    let visuals = ui.visuals();
    let weak = visuals.weak_text_color();
    let (mark, mark_color) = match message.direction {
        Direction::Sent => (SENT_MARK, visuals.hyperlink_color),
        Direction::Received => (RECEIVED_MARK, success_color(ui)),
    };
    let mut job = LayoutJob::default();
    append(
        &mut job,
        &format!("{number:>width$}{COLUMN_GAP}"),
        &font,
        weak,
    );
    append(&mut job, mark, &font, mark_color);
    let time = format!("{TIME_PREFIX}{}", ElapsedText(message.at));
    append(
        &mut job,
        &format!("{COLUMN_GAP}{time:<TIME_WIDTH$}{COLUMN_GAP}"),
        &font,
        weak,
    );
    append(
        &mut job,
        &preview(&message.payload),
        &font,
        visuals.text_color(),
    );
    let response = ui.add(Label::new(job).truncate().sense(Sense::click()));
    if let Some(text) = message.text() {
        if response.on_hover_text(COPY_HINT).clicked() {
            actions.push(Action::CopyText(text.to_string()));
        }
    }
}

fn preview(payload: &SocketPayload) -> String {
    match payload {
        SocketPayload::Text(text) => {
            let shown: String = text.chars().take(MAX_SHOWN_CHARS).collect();
            shown.replace('\n', LINE_BREAK_MARK)
        }
        SocketPayload::Binary(bytes) => format!("{BINARY_PREFIX} {}", ByteSize(bytes.len())),
    }
}

fn append(job: &mut LayoutJob, text: &str, font: &FontId, color: Color32) {
    job.append(text, 0.0, TextFormat::simple(font.clone(), color));
}
