use eframe::egui::{Label, RichText, ScrollArea, Ui};
use requestor_core::script::{CheckOutcome, VariableWrite, Written};

use crate::response::ResponseView;
use crate::widgets::success_color;

const PASSED_MARK: &str = "✓";
const FAILED_MARK: &str = "✗";
const NO_CHECKS_TEXT: &str = "The scripts ran without expect lines.";
const SAVED_TITLE: &str = "Variables";
const SAVED_TEXT: &str = "saved";
const REMOVED_TEXT: &str = "cleared";
const STOPPED_TITLE: &str = "Script stopped:";
const NOT_SAVED_TEXT: &str = "not saved:";
const SECTION_GAP: f32 = 12.0;

pub fn show(ui: &mut Ui, view: &ResponseView) {
    let Some(report) = &view.report else {
        return;
    };
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        if report.checks.is_empty() {
            ui.weak(NO_CHECKS_TEXT);
        }
        for outcome in &report.checks {
            outcome_row(ui, outcome);
        }
        if let Some(failure) = &report.failure {
            ui.add_space(SECTION_GAP);
            ui.horizontal(|ui| {
                ui.strong(STOPPED_TITLE);
                ui.label(failure);
            });
        }
        if !report.writes.is_empty() {
            ui.add_space(SECTION_GAP);
            ui.strong(SAVED_TITLE);
            for write in &report.writes {
                write_row(ui, write);
            }
        }
    });
}

fn outcome_row(ui: &mut Ui, outcome: &CheckOutcome) {
    ui.horizontal(|ui| {
        match &outcome.failure {
            None => ui.label(RichText::new(PASSED_MARK).color(success_color(ui))),
            Some(_) => ui.label(RichText::new(FAILED_MARK).color(ui.visuals().error_fg_color)),
        };
        ui.add(Label::new(&outcome.title).selectable(true));
        if let Some(failure) = &outcome.failure {
            ui.weak(failure);
        }
    });
}

fn write_row(ui: &mut Ui, write: &VariableWrite) {
    ui.horizontal(|ui| {
        let variable = RichText::new(&write.variable).monospace();
        match &write.value {
            Ok(written) => {
                ui.label(RichText::new(PASSED_MARK).color(success_color(ui)));
                ui.label(variable);
                ui.weak(match written {
                    Written::Value(_) => SAVED_TEXT,
                    Written::Removed => REMOVED_TEXT,
                });
            }
            Err(reason) => {
                ui.label(RichText::new(FAILED_MARK).color(ui.visuals().error_fg_color));
                ui.label(variable);
                ui.weak(format!("{NOT_SAVED_TEXT} {reason}"));
            }
        }
    });
}
