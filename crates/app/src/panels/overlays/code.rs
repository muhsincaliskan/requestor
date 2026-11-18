use std::sync::Arc;

use eframe::egui::text::LayoutJob;
use eframe::egui::{vec2, Context, Galley, ScrollArea, TextBuffer, TextEdit, TextStyle, Ui};
use requestor_core::prepared::CodeLanguage;

use super::{button_row, dialog};
use crate::state::{Action, CodeForm};
use crate::widgets::action_button;

const DIALOG_ID: &str = "code";
const TITLE: &str = "Code";
const SECRETS_LABEL: &str = "Include secrets";
const SECRETS_HINT: &str = "Off: tokens, passwords and secret variables show as <redacted>.";
const COPY_LABEL: &str = "Copy and close";
const CLOSE_LABEL: &str = "Close";
const CODE_SALT: &str = "generated-code";
const DIALOG_WIDTH: f32 = 640.0;
const CODE_HEIGHT: f32 = 360.0;

pub fn show(ctx: &Context, form: &CodeForm, actions: &mut Vec<Action>) -> bool {
    dialog(ctx, DIALOG_ID, TITLE, |ui| {
        ui.set_width(DIALOG_WIDTH);
        options_row(ui, form, actions);
        code_view(ui, &form.code);
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

fn options_row(ui: &mut Ui, form: &CodeForm, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        for language in CodeLanguage::ALL {
            let selected = form.language == language;
            let clicked = ui
                .selectable_label(selected, language.label())
                .on_hover_text(language.library())
                .clicked();
            if clicked && !selected {
                actions.push(form.with_language(language));
            }
        }
        ui.separator();
        let mut include_secrets = form.include_secrets;
        let toggled = ui
            .checkbox(&mut include_secrets, SECRETS_LABEL)
            .on_hover_text(SECRETS_HINT)
            .changed();
        if toggled {
            actions.push(form.with_secrets(include_secrets));
        }
    });
}

fn code_view(ui: &mut Ui, code: &str) {
    ScrollArea::both()
        .id_salt(CODE_SALT)
        .max_height(CODE_HEIGHT)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add(
                TextEdit::multiline(&mut &*code)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .min_size(vec2(0.0, CODE_HEIGHT))
                    .layouter(&mut unwrapped),
            );
        });
}

fn unwrapped(ui: &Ui, text: &dyn TextBuffer, _wrap_width: f32) -> Arc<Galley> {
    let font = TextStyle::Monospace.resolve(ui.style());
    let job = LayoutJob::simple(
        text.as_str().to_string(),
        font,
        ui.visuals().text_color(),
        f32::INFINITY,
    );
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}
