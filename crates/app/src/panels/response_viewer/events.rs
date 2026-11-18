use eframe::egui::text::LayoutJob;
use eframe::egui::{Color32, FontId, Label, ScrollArea, Sense, TextFormat, TextStyle, Ui};
use requestor_core::event_stream::{EventLog, ServerEvent};

use super::body::digits;
use crate::state::Action;

const SCROLL_ID: &str = "response-events";
const NO_EVENTS_TEXT: &str = "No events yet.";
const COPY_HINT: &str = "Click to copy the data";
const LINE_BREAK_MARK: &str = " ⏎ ";
const DROPPED_SUFFIX: &str = "earlier events are not shown";
const ID_PREFIX: &str = "id ";
const COLUMN_GAP: &str = "  ";

pub(super) fn list(ui: &mut Ui, log: Option<&EventLog>, actions: &mut Vec<Action>) {
    let Some(log) = log.filter(|log| log.kept() > 0) else {
        ui.weak(NO_EVENTS_TEXT);
        return;
    };
    if log.dropped() > 0 {
        ui.weak(format!("{} {DROPPED_SUFFIX}", log.dropped()));
    }
    let row_height = ui.text_style_height(&TextStyle::Monospace);
    let first_number = log.dropped() + 1;
    let width = digits(log.total());
    ScrollArea::vertical()
        .id_salt(SCROLL_ID)
        .auto_shrink(false)
        .stick_to_bottom(true)
        .show_rows(ui, row_height, log.kept(), |ui, rows| {
            for index in rows {
                if let Some(event) = log.get(index) {
                    row(ui, first_number + index, width, event, actions);
                }
            }
        });
}

fn row(ui: &mut Ui, number: usize, width: usize, event: &ServerEvent, actions: &mut Vec<Action>) {
    let font = TextStyle::Monospace.resolve(ui.style());
    let visuals = ui.visuals();
    let weak = visuals.weak_text_color();
    let mut job = LayoutJob::default();
    append(
        &mut job,
        &format!("{number:>width$}{COLUMN_GAP}"),
        &font,
        weak,
    );
    append(&mut job, event.kind(), &font, visuals.hyperlink_color);
    if let Some(id) = &event.id {
        append(
            &mut job,
            &format!("{COLUMN_GAP}{ID_PREFIX}{id}"),
            &font,
            weak,
        );
    }
    let data = event.data.replace('\n', LINE_BREAK_MARK);
    append(
        &mut job,
        &format!("{COLUMN_GAP}{data}"),
        &font,
        visuals.text_color(),
    );
    let response = ui
        .add(Label::new(job).truncate().sense(Sense::click()))
        .on_hover_text(COPY_HINT);
    if response.clicked() {
        actions.push(Action::CopyText(event.data.clone()));
    }
}

fn append(job: &mut LayoutJob, text: &str, font: &FontId, color: Color32) {
    job.append(text, 0.0, TextFormat::simple(font.clone(), color));
}
