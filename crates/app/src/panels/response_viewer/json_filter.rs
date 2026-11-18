use eframe::egui::{RichText, TextEdit, TextStyle, Ui};

use crate::response::ResponseView;
use crate::widgets::variable_text_edit::FIELD_MARGIN;

const FILTER_HINT: &str = "Filter with JSONPath, e.g. $.items[*].id";
const FILTER_WIDTH: f32 = 340.0;
const NOT_JSON_TEXT: &str = "The body is not valid JSON";
const ONE_RESULT: &str = "1 result";
const RESULTS_SUFFIX: &str = "results";

pub fn show(ui: &mut Ui, view: &mut ResponseView) {
    ui.horizontal(|ui| {
        let mut text = view.inspection().filter_text().to_string();
        let field = ui.add(
            TextEdit::singleline(&mut text)
                .hint_text(FILTER_HINT)
                .font(TextStyle::Monospace)
                .margin(FIELD_MARGIN)
                .desired_width(FILTER_WIDTH),
        );
        if field.changed() {
            view.set_filter(text);
        }
        outcome(ui, view);
    });
}

fn outcome(ui: &mut Ui, view: &mut ResponseView) {
    if let Some(error) = view.inspection().error() {
        ui.label(RichText::new(error.to_string()).color(ui.visuals().error_fg_color));
        return;
    }
    if !view.inspection().is_filtering() {
        return;
    }
    match view.filter_result_count() {
        Some(1) => ui.weak(ONE_RESULT),
        Some(count) => ui.weak(format!("{count} {RESULTS_SUFFIX}")),
        None => ui.weak(NOT_JSON_TEXT),
    };
}
