use eframe::egui::text::LayoutJob;
use eframe::egui::{
    vec2, Align, Key, KeyboardShortcut, Label, Layout, Modifiers, Rect, ScrollArea, TextEdit,
    TextFormat, TextStyle, TextWrapMode, Ui,
};
use requestor_core::response::ResponseFormat;

use super::readout::ByteSize;
use super::{json_filter, tree};
use crate::response::{BodyLayout, BodySearch, ResponseBuffer, ResponseView};
use crate::state::Action;
use crate::widgets::variable_text_edit::FIELD_MARGIN;
use crate::widgets::{append_row, icon_button, layout_row, SyntaxPalette};

const COPY_LABEL: &str = "Copy";
const SCROLL_ID: &str = "response-body";
const GUTTER_GAP: &str = "  ";
const FIND_HINT: &str = "Find";
const FIND_WIDTH: f32 = 180.0;
const PREVIOUS_LABEL: &str = "‹";
const NEXT_LABEL: &str = "›";
const PREVIOUS_HINT: &str = "Previous match";
const NEXT_HINT: &str = "Next match";
const NO_MATCHES: &str = "No matches";
const ROWS_ABOVE_MATCH: f32 = 3.0;
const FIND_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::F);

pub fn show(ui: &mut Ui, view: &mut ResponseView, actions: &mut Vec<Action>) {
    let tree = shows_tree(view) && view.json_tree().is_some();
    toolbar(ui, view, tree, actions);
    if view.is_inspectable() {
        json_filter::show(ui, view);
    }
    ui.separator();
    let format = view.format;
    if format == ResponseFormat::Binary {
        let font = TextStyle::Monospace.resolve(ui.style());
        let palette = SyntaxPalette::for_mode(ui.visuals().dark_mode);
        let notice = layout_row(view.display().row(0), format, &palette, &font);
        ui.add(Label::new(notice).selectable(true));
        return;
    }
    if tree {
        if let Some(root) = view.json_tree() {
            tree::show(ui, &root);
            return;
        }
    }
    let (buffer, search) = view.searched_display();
    rows(ui, buffer, format, search);
}

fn shows_tree(view: &ResponseView) -> bool {
    view.is_inspectable() && view.layout() == BodyLayout::Tree
}

fn layout_choice(ui: &mut Ui, view: &mut ResponseView) {
    if !view.format.has_layout() {
        return;
    }
    let selected = match (view.layout(), view.is_inspectable()) {
        (BodyLayout::Tree, false) => BodyLayout::Pretty,
        (layout, _) => layout,
    };
    for choice in BodyLayout::ALL {
        if choice == BodyLayout::Tree && !view.is_inspectable() {
            continue;
        }
        if ui
            .selectable_label(selected == choice, choice.label())
            .clicked()
        {
            view.set_layout(choice);
        }
    }
}

fn toolbar(ui: &mut Ui, view: &mut ResponseView, tree: bool, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        layout_choice(ui, view);
        let size = ByteSize(view.size_bytes());
        if tree {
            if ui.button(COPY_LABEL).clicked() {
                actions.push(Action::CopyText(view.display().text().to_string()));
            }
            ui.weak(size.to_string());
            return;
        }
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
    if icon_button(ui, NEXT_LABEL, NEXT_HINT).clicked() {
        search.step(true);
    }
    if icon_button(ui, PREVIOUS_LABEL, PREVIOUS_HINT).clicked() {
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
            .margin(FIELD_MARGIN)
            .desired_width(FIND_WIDTH),
    );
    if ui.input_mut(|input| input.consume_shortcut(&FIND_SHORTCUT)) {
        field.request_focus();
    }
    let field = field.on_hover_text(ui.ctx().format_shortcut(&FIND_SHORTCUT));
    if field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
        search.step(!ui.input(|input| input.modifiers.shift));
        field.request_focus();
    }
}

pub(super) fn digits(count: usize) -> usize {
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
