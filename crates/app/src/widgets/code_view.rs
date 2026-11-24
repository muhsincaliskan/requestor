use crate::widgets::check_box;
use std::sync::Arc;

use eframe::egui::text::LayoutJob;
use eframe::egui::{vec2, Galley, ScrollArea, TextBuffer, TextEdit, TextStyle, Ui};
use requestor_core::prepared::CodeLanguage;

const SECRETS_LABEL: &str = "Include secrets";
const SECRETS_HINT: &str = "Off: tokens, passwords and secret variables show as <redacted>.";
const CODE_SALT: &str = "generated-code";
const PREVIEW_LIMIT_BYTES: usize = 256 * 1024;
const PREVIEW_CUT_NOTE: &str = "\n… The preview stops here. Copy takes the whole code.";

pub struct CodeChoice {
    pub language: CodeLanguage,
    pub include_secrets: bool,
}

impl CodeChoice {
    pub fn show(mut self, ui: &mut Ui) -> Option<CodeChoice> {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            for language in CodeLanguage::ALL {
                let selected = self.language == language;
                let clicked = ui
                    .selectable_label(selected, language.label())
                    .on_hover_text(language.library())
                    .clicked();
                if clicked && !selected {
                    self.language = language;
                    changed = true;
                }
            }
            ui.separator();
            changed |= check_box(ui, &mut self.include_secrets, SECRETS_LABEL)
                .on_hover_text(SECRETS_HINT)
                .changed();
        });
        changed.then_some(self)
    }
}

pub fn code_view(ui: &mut Ui, code: &str, height: f32) {
    let shown = preview(code);
    ScrollArea::both()
        .id_salt(CODE_SALT)
        .max_height(height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add(
                TextEdit::multiline(&mut shown.as_ref())
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .min_size(vec2(0.0, height))
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

fn preview(code: &str) -> std::borrow::Cow<'_, str> {
    if code.len() <= PREVIEW_LIMIT_BYTES {
        return code.into();
    }
    let mut end = PREVIEW_LIMIT_BYTES;
    while !code.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{PREVIEW_CUT_NOTE}", &code[..end]).into()
}
