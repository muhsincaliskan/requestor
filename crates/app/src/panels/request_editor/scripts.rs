mod editor;
mod javascript;
mod problems;

use eframe::egui::{Align, Layout, RichText, Sense, Stroke, Ui};
use requestor_core::script::Phase;

use crate::state::{RequestTab, ScriptView};
use crate::widgets::capsule::Capsule;
use crate::widgets::variable_text_edit::VariableHints;

const PHASES: [(Phase, &str); 2] = [
    (Phase::BeforeRequest, "Before request"),
    (Phase::AfterResponse, "After response"),
];
const VIEWS: [(ScriptView, &str); 2] = [
    (ScriptView::Script, "Script"),
    (ScriptView::JavaScript, "JavaScript"),
];
const UNDERLINE_WIDTH: f32 = 2.0;
const UNDERLINE_GAP: f32 = 3.0;
const TAB_GAP: f32 = 16.0;

pub fn show(ui: &mut Ui, open: &mut RequestTab, hints: &VariableHints) -> bool {
    header(ui, open);
    ui.add_space(ui.spacing().item_spacing.y);
    let pane = &mut open.script_pane;
    let draft = open.draft.scripts.of_phase_mut(pane.phase);
    match pane.view {
        ScriptView::Script => editor::show(ui, pane, draft, hints),
        ScriptView::JavaScript => javascript::show(ui, pane, draft),
    }
}

fn header(ui: &mut Ui, open: &mut RequestTab) {
    ui.horizontal(|ui| {
        for (phase, label) in PHASES {
            let draft = open.draft.scripts.of_phase_mut(phase);
            let marked = match draft.has_problems() {
                true => format!("{label} !"),
                false => label.to_string(),
            };
            if section_tab(ui, open.script_pane.phase == phase, &marked).clicked() {
                open.script_pane.select_phase(phase);
            }
            ui.add_space(TAB_GAP);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let mut view = open.script_pane.view;
            if ui.add(Capsule::new(&mut view, VIEWS)).changed() {
                open.script_pane.select_view(view);
            }
        });
    });
}

fn section_tab(ui: &mut Ui, selected: bool, label: &str) -> eframe::egui::Response {
    let text = match selected {
        true => RichText::new(label).strong(),
        false => RichText::new(label).color(ui.visuals().weak_text_color()),
    };
    let response = ui.add(eframe::egui::Label::new(text).sense(Sense::click()));
    if selected {
        let rect = response.rect;
        let y = rect.bottom() + UNDERLINE_GAP;
        let stroke = Stroke::new(UNDERLINE_WIDTH, ui.visuals().selection.bg_fill);
        ui.painter().hline(rect.x_range(), y, stroke);
    }
    response.on_hover_cursor(eframe::egui::CursorIcon::PointingHand)
}
