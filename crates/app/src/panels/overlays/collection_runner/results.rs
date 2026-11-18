use eframe::egui::{Frame, Label, RichText, ScrollArea, Ui};
use requestor_core::collection::RunStepResult;
use requestor_core::response::StatusClass;

use super::LIST_HEIGHT;
use crate::state::RunnerForm;
use crate::widgets::{method_color, status_color};

const RESULTS_LABEL: &str = "Results";
const FAILED_TEXT: &str = "Failed";
const NO_TIME_TEXT: &str = "–";
const RESULT_COLUMNS: [&str; 6] = ["#", "Method", "Request", "Status", "Time", "Tests"];
const COLUMN_WIDTHS: [f32; 6] = [34.0, 62.0, 210.0, 62.0, 84.0, 56.0];

pub(super) fn show(ui: &mut Ui, form: &RunnerForm) {
    let (done, total) = form.progress();
    let failed = form.failed_count();
    ui.horizontal(|ui| {
        ui.strong(RESULTS_LABEL);
        ui.weak(format!(
            "{done}/{total} · {} passed · {failed} failed",
            form.passed_count()
        ));
    });
    let height = ui.spacing().interact_size.y;
    ui.horizontal(|ui| {
        for (title, width) in RESULT_COLUMNS.into_iter().zip(COLUMN_WIDTHS) {
            cell(ui, RichText::new(title).weak(), width, height);
        }
    });
    ScrollArea::vertical()
        .id_salt(RESULTS_LABEL)
        .max_height(LIST_HEIGHT)
        .stick_to_bottom(true)
        .show_rows(ui, height, done, |ui, rows| {
            for index in rows {
                let fill = if index % 2 == 0 {
                    ui.visuals().faint_bg_color
                } else {
                    eframe::egui::Color32::TRANSPARENT
                };
                Frame::new().fill(fill).show(ui, |ui| {
                    row(ui, &form.results()[index], height);
                });
            }
        });
}

fn row(ui: &mut Ui, result: &RunStepResult, height: f32) {
    ui.horizontal(|ui| {
        let method = RichText::new(&result.method)
            .color(method_color(ui, &result.method))
            .small();
        let (status, color) = result.status.map_or_else(
            || (FAILED_TEXT.to_string(), ui.visuals().error_fg_color),
            |status| {
                (
                    status.to_string(),
                    status_color(ui, StatusClass::of(status)),
                )
            },
        );
        let time = result.status.map_or_else(
            || RichText::new(NO_TIME_TEXT).weak(),
            |_| RichText::new(format!("{} ms", result.elapsed.as_millis())),
        );
        let tests = RichText::new(format!("{}/{}", result.checks_passed, result.checks_total));
        let tests = if result.checks_passed < result.checks_total {
            tests.color(ui.visuals().error_fg_color)
        } else {
            tests.weak()
        };
        let columns = [
            RichText::new(result.iteration.to_string()),
            method,
            RichText::new(&result.name),
            RichText::new(status).color(color),
            time,
            tests,
        ];
        for ((index, text), width) in columns.into_iter().enumerate().zip(COLUMN_WIDTHS) {
            let response = cell(ui, text, width, height);
            if index == STATUS_COLUMN {
                if let Some(error) = &result.error {
                    response.on_hover_text(error);
                }
            }
        }
    });
}

const STATUS_COLUMN: usize = 3;

fn cell(ui: &mut Ui, text: RichText, width: f32, height: f32) -> eframe::egui::Response {
    ui.add_sized([width, height], Label::new(text).truncate())
}
