use eframe::egui::{Label, RichText, ScrollArea, Ui};
use requestor_core::checks::{CheckOutcome, Extracted};

use crate::response::ResponseView;
use crate::widgets::success_color;

const PASSED_MARK: &str = "✓";
const FAILED_MARK: &str = "✗";
const NO_CHECKS_TEXT: &str = "This request has no checks, only saved values.";
const SAVED_TITLE: &str = "Saved values";
const SAVED_TEXT: &str = "saved";
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
        if !report.extracted.is_empty() {
            ui.add_space(SECTION_GAP);
            ui.strong(SAVED_TITLE);
            for extracted in &report.extracted {
                extracted_row(ui, extracted);
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

fn extracted_row(ui: &mut Ui, extracted: &Extracted) {
    ui.horizontal(|ui| {
        let variable = RichText::new(&extracted.extraction.variable).monospace();
        match &extracted.value {
            Ok(_) => {
                ui.label(RichText::new(PASSED_MARK).color(success_color(ui)));
                ui.label(variable);
                ui.weak(SAVED_TEXT);
            }
            Err(reason) => {
                ui.label(RichText::new(FAILED_MARK).color(ui.visuals().error_fg_color));
                ui.label(variable);
                ui.weak(format!("{NOT_SAVED_TEXT} {reason}"));
            }
        }
    });
}
