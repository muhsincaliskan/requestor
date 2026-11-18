use eframe::egui::{Align, Layout, RichText, Ui};

use crate::state::{Action, CodePanel};
use crate::widgets::close_button;
use crate::widgets::code_view::{code_view, CodeChoice};

const TITLE: &str = "Code";
const COPY_LABEL: &str = "Copy";
const CLOSE_HINT: &str = "Hide code";

pub fn show(ui: &mut Ui, panel: &CodePanel, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(TITLE).strong());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if close_button(ui, CLOSE_HINT).clicked() {
                actions.push(Action::ToggleCodePanel);
            }
            let code = panel.code.as_ref().ok();
            let copy = ui.add_enabled_ui(code.is_some(), |ui| ui.button(COPY_LABEL));
            if let (true, Some(code)) = (copy.inner.clicked(), code) {
                actions.push(Action::CopyText(code.clone()));
            }
        });
    });
    let choice = CodeChoice {
        language: panel.language,
        include_secrets: panel.include_secrets,
    };
    if let Some(choice) = choice.show(ui) {
        actions.push(panel.with_options(choice.language, choice.include_secrets));
    }
    match &panel.code {
        Ok(code) => code_view(ui, code, ui.available_height()),
        Err(reason) => {
            ui.weak(reason);
        }
    }
}
