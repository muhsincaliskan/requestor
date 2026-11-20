use eframe::egui::{vec2, Grid, Rect, RichText, Ui, UiBuilder};

use crate::shortcuts;
use crate::state::{Action, Overlay};
use crate::widgets::action_button;

const TITLE: &str = "Start a request";
const GUIDANCE: [&str; 2] = [
    "Type a URL or paste a cURL command into a new request.",
    "Drop a Postman collection, environment, OpenAPI or WSDL file anywhere to import it.",
];
const NEW_REQUEST_LABEL: &str = "New request";
const IMPORT_LABEL: &str = "Import…";
const MIGRATION_LABEL: &str = "Import from Postman…";
const SHORTCUT_GRID_ID: &str = "welcome-shortcuts";
const SHOWN_SHORTCUTS: usize = 4;
const TOP_SHARE: f32 = 0.22;
const COLUMN_WIDTH: f32 = 460.0;
const SECTION_GAP: f32 = 16.0;

pub fn show(ui: &mut Ui, actions: &mut Vec<Action>) {
    let available = ui.available_rect_before_wrap();
    let width = COLUMN_WIDTH.min(available.width());
    let top_left = available.center_top() + vec2(-width / 2.0, available.height() * TOP_SHARE);
    let column = Rect::from_min_size(top_left, vec2(width, available.height()));
    ui.scope_builder(UiBuilder::new().max_rect(column), |ui| {
        ui.heading(TITLE);
        ui.add_space(SECTION_GAP);
        for line in GUIDANCE {
            ui.weak(line);
        }
        ui.add_space(SECTION_GAP);
        buttons(ui, actions);
        ui.add_space(SECTION_GAP);
        shortcut_list(ui);
    });
}

fn buttons(ui: &mut Ui, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        if action_button::primary(ui, NEW_REQUEST_LABEL).clicked() {
            actions.push(Action::NewTab);
        }
        if ui.button(IMPORT_LABEL).clicked() {
            actions.push(Action::Show(Overlay::Import(Default::default())));
        }
        if ui.button(MIGRATION_LABEL).clicked() {
            actions.push(Action::Show(Overlay::PostmanMigration));
        }
    });
}

fn shortcut_list(ui: &mut Ui) {
    Grid::new(SHORTCUT_GRID_ID).show(ui, |ui| {
        for shortcut in shortcuts::ALL.iter().take(SHOWN_SHORTCUTS) {
            ui.weak(shortcut.label);
            ui.label(RichText::new(ui.ctx().format_shortcut(&shortcut.keys)).monospace());
            ui.end_row();
        }
    });
}
