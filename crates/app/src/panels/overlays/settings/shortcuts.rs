use eframe::egui::{Grid, RichText, Ui};

use crate::shortcuts;

const GRID_ID: &str = "settings-shortcuts";
const COLUMN_SPACING: f32 = 48.0;

pub fn show(ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    Grid::new(GRID_ID)
        .num_columns(2)
        .striped(true)
        .spacing([COLUMN_SPACING, ui.spacing().item_spacing.y])
        .show(ui, |ui| {
            for shortcut in &shortcuts::ALL {
                ui.label(shortcut.label);
                ui.label(RichText::new(shortcut.keys_text(&ctx)).monospace());
                ui.end_row();
            }
        });
}
