use eframe::egui::{RichText, Ui};
use requestor_core::script::{ScriptError, Unconverted};

pub fn script_errors(ui: &mut Ui, errors: &[ScriptError]) {
    let color = ui.visuals().error_fg_color;
    for error in errors {
        ui.colored_label(color, format!("Line {}: {}", error.line, error.message));
    }
}

pub fn unconverted_lines(ui: &mut Ui, unconverted: &Unconverted) {
    let color = ui.visuals().error_fg_color;
    for line in &unconverted.lines {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(color, format!("Line {}:", line.line));
            ui.label(RichText::new(&line.text).monospace());
            ui.weak(&line.reason);
        });
    }
}
