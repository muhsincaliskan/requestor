use eframe::egui::text::LayoutJob;
use eframe::egui::{
    vec2, Align, Button, Key, Label, Layout, Rect, ScrollArea, TextEdit, TextFormat, TextStyle,
    TextWrapMode, Ui,
};
use requestor_core::response::ResponseFormat;

use super::readout::ByteSize;
use crate::response::{BodySearch, ResponseBuffer, ResponseView};
use crate::state::Action;
use crate::widgets::{append_row, layout_row, SyntaxPalette};

const PRETTY_LABEL: &str = "Pretty";
const RAW_LABEL: &str = "Raw";
const COPY_LABEL: &str = "Copy";
const SCROLL_ID: &str = "response-body";
const GUTTER_GAP: &str = "  ";
const FIND_HINT: &str = "Find";
const FIND_WIDTH: f32 = 180.0;
const PREVIOUS_LABEL: &str = "‹";
const NEXT_LABEL: &str = "›";
const NO_MATCHES: &str = "No matches";
const ROWS_ABOVE_MATCH: f32 = 3.0;

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
    let (buffer, search) = view.searched_display();
    rows(ui, buffer, format, search);
}

fn toolbar(ui: &mut Ui, view: &mut ResponseView, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        if view.format == ResponseFormat::Json {
            let mut pretty = view.is_pretty();
            ui.selectable_value(&mut pretty, true, PRETTY_LABEL);
            ui.selectable_value(&mut pretty, false, RAW_LABEL);
            view.set_pretty(pretty);
        }
        let size = ByteSize(view.size_bytes());
        let searchable = view.format != ResponseFormat::Binary;
        let (buffer, search) = view.searched_display();
        if ui.button(COPY_LABEL).clicked() {
            actions.push(Action::CopyText(buffer.text().to_string()));
        }
        ui.weak(format!("{} lines · {size}", buffer.row_count()));
        if searchable {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                find_box(ui, search)
            });
        }
    });
}

fn find_box(ui: &mut Ui, search: &mut BodySearch) {
    if ui.add(Button::new(NEXT_LABEL).frame(false)).clicked() {
        search.step(true);
    }
    if ui.add(Button::new(PREVIOUS_LABEL).frame(false)).clicked() {
        search.step(false);
    }
    match search.position() {
        Some((_, 0)) => {
            ui.weak(NO_MATCHES);
        }
        Some((current, count)) => {
            ui.weak(format!("{current}/{count}"));
        }
        None => {}
    }
    let field = ui.add(
        TextEdit::singleline(&mut search.query)
            .hint_text(FIND_HINT)
            .desired_width(FIND_WIDTH),
    );
    if field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
        search.step(!ui.input(|input| input.modifiers.shift));
        field.request_focus();
    }
}

fn digits(count: usize) -> usize {
    count.max(1).ilog10() as usize + 1
}

fn rows(ui: &mut Ui, buffer: &ResponseBuffer, format: ResponseFormat, search: &mut BodySearch) {
    let font = TextStyle::Monospace.resolve(ui.style());
    let row_height = ui.text_style_height(&TextStyle::Monospace);
    let palette = SyntaxPalette::for_mode(ui.visuals().dark_mode);
    let width = digits(buffer.row_count());
    let gutter_color = ui.visuals().weak_text_color();
    let mut area = ScrollArea::both().id_salt(SCROLL_ID).auto_shrink(false);
    if let Some(target) = search.take_scroll_target() {
        let spaced = row_height + ui.spacing().item_spacing.y;
        area = area.vertical_scroll_offset((target as f32 - ROWS_ABOVE_MATCH).max(0.0) * spaced);
    }
    area.show_rows(ui, row_height, buffer.row_count(), |ui, visible| {
        for index in visible {
            if search.is_match(index) {
                mark_row(ui, row_height, search.current_row() == Some(index));
            }
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

fn mark_row(ui: &Ui, row_height: f32, current: bool) {
    let rect = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), row_height));
    let visuals = ui.visuals();
    let fill = if current {
        visuals.selection.bg_fill
    } else {
        visuals.faint_bg_color
    };
    ui.painter().rect_filled(rect, 0.0, fill);
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
