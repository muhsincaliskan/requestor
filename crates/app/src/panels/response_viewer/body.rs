use eframe::egui::text::LayoutJob;
use eframe::egui::{Label, ScrollArea, TextFormat, TextStyle, TextWrapMode, Ui};
use requestor_core::response::ResponseFormat;

use super::readout::ByteSize;
use crate::response::{ResponseBuffer, ResponseView};
use crate::state::Action;
use crate::widgets::{append_row, layout_row, SyntaxPalette};

const PRETTY_LABEL: &str = "Pretty";
const RAW_LABEL: &str = "Raw";
const COPY_LABEL: &str = "Copy";
const SCROLL_ID: &str = "response-body";
const GUTTER_GAP: &str = "  ";

pub fn show(ui: &mut Ui, view: &mut ResponseView, actions: &mut Vec<Action>) {
    toolbar(ui, view, actions);
    ui.separator();
    let format = view.format;
    if format == ResponseFormat::Binary {
        let font = TextStyle::Monospace.resolve(ui.style());
        let palette = SyntaxPalette::for_mode(ui.visuals().dark_mode);
        let notice = layout_row(view.display().row(0), format, &palette, &font);
        ui.add(Label::new(notice).selectable(true));
        return;
    }
    rows(ui, view.display(), format);
}

fn toolbar(ui: &mut Ui, view: &mut ResponseView, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        if view.format == ResponseFormat::Json {
            let mut pretty = view.is_pretty();
            ui.selectable_value(&mut pretty, true, PRETTY_LABEL);
            ui.selectable_value(&mut pretty, false, RAW_LABEL);
            view.set_pretty(pretty);
        }
        if ui.button(COPY_LABEL).clicked() {
            actions.push(Action::CopyText(view.display().text().to_string()));
        }
        let lines = view.display().row_count();
        ui.weak(format!("{lines} lines · {}", ByteSize(view.size_bytes())));
    });
}

fn digits(count: usize) -> usize {
    count.max(1).ilog10() as usize + 1
}

fn rows(ui: &mut Ui, buffer: &ResponseBuffer, format: ResponseFormat) {
    let font = TextStyle::Monospace.resolve(ui.style());
    let row_height = ui.text_style_height(&TextStyle::Monospace);
    let palette = SyntaxPalette::for_mode(ui.visuals().dark_mode);
    let width = digits(buffer.row_count());
    let gutter_color = ui.visuals().weak_text_color();
    ScrollArea::both()
        .id_salt(SCROLL_ID)
        .auto_shrink(false)
        .show_rows(ui, row_height, buffer.row_count(), |ui, visible| {
            for index in visible {
                let mut job = LayoutJob::default();
                let gutter = TextFormat::simple(font.clone(), gutter_color);
                job.append(&format!("{:>width$}{GUTTER_GAP}", index + 1), 0.0, gutter);
                append_row(&mut job, buffer.row(index), format, &palette, &font);
                ui.add(
                    Label::new(job)
                        .wrap_mode(TextWrapMode::Extend)
                        .selectable(false),
                );
            }
        });
}

#[cfg(test)]
mod tests {
    use super::digits;

    #[test]
    fn gutter_width_follows_the_row_count() {
        assert_eq!(digits(0), 1);
        assert_eq!(digits(9), 1);
        assert_eq!(digits(10), 2);
        assert_eq!(digits(2000), 4);
    }
}
