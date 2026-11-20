use eframe::egui::{Frame, Margin, RichText, Ui};

use super::NEW_COLLECTION_LABEL;
use crate::panels::overlays::postman_migration::TITLE as MIGRATION_LABEL;
use crate::state::{Action, Overlay};
use crate::widgets::action_button;

const HEADING: &str = "Coming from Postman?";
const BODY: &str = "Bring every collection and environment over in one step.";
const OTHER_FORMATS_LABEL: &str = "Other formats…";
const OTHER_FORMATS_HINT: &str = "OpenAPI, WSDL, cURL or a raw HTTP request";
const CARD_MARGIN: i8 = 10;
const CARD_RADIUS: u8 = 6;
const LINE_GAP: f32 = 6.0;

pub(super) fn show(ui: &mut Ui, actions: &mut Vec<Action>) {
    Frame::group(ui.style())
        .inner_margin(Margin::same(CARD_MARGIN))
        .corner_radius(CARD_RADIUS)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(HEADING).strong());
            ui.weak(BODY);
            ui.add_space(LINE_GAP);
            if action_button::primary(ui, MIGRATION_LABEL).clicked() {
                actions.push(Action::OpenPostmanMigration);
            }
            ui.add_space(LINE_GAP);
            if ui.link(NEW_COLLECTION_LABEL).clicked() {
                actions.push(Action::NewCollection);
            }
            if ui
                .link(OTHER_FORMATS_LABEL)
                .on_hover_text(OTHER_FORMATS_HINT)
                .clicked()
            {
                actions.push(Action::Show(Overlay::Import(Default::default())));
            }
        });
}
