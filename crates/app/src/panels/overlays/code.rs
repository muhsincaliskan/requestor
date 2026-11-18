use eframe::egui::Context;

use super::{button_row, dialog};
use crate::state::{Action, CodeForm};
use crate::widgets::action_button;
use crate::widgets::code_view::{code_view, CodeChoice};

const DIALOG_ID: &str = "code";
const TITLE: &str = "Code";
const COPY_LABEL: &str = "Copy and close";
const CLOSE_LABEL: &str = "Close";
const DIALOG_WIDTH: f32 = 640.0;
const CODE_HEIGHT: f32 = 360.0;

pub fn show(ctx: &Context, form: &CodeForm, actions: &mut Vec<Action>) -> bool {
    dialog(ctx, DIALOG_ID, TITLE, |ui| {
        ui.set_width(DIALOG_WIDTH);
        let choice = CodeChoice {
            language: form.language,
            include_secrets: form.include_secrets,
        };
        if let Some(choice) = choice.show(ui) {
            actions.push(form.with_options(choice.language, choice.include_secrets));
        }
        code_view(ui, &form.code, CODE_HEIGHT);
        button_row(ui, |ui| {
            let copied = action_button::primary(ui, COPY_LABEL).clicked();
            if copied {
                actions.push(Action::CopyText(form.code.clone()));
            }
            let closed = ui.button(CLOSE_LABEL).clicked();
            !(copied || closed)
        })
    })
}
