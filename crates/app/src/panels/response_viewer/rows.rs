use std::ops::Range;
use std::sync::Arc;

use eframe::egui::text::{CCursor, LayoutJob};
use eframe::egui::{
    vec2, Color32, CursorIcon, Event, FontId, Galley, Key, KeyboardShortcut, Modifiers, Pos2, Rect,
    Response, ScrollArea, Sense, TextStyle, Ui,
};
use requestor_core::response::ResponseFormat;

use super::body::digits;
use crate::response::{BodyDisplay, BodySelection, ResponseBuffer};
use crate::widgets::{append_row, SyntaxPalette};

const SCROLL_ID: &str = "response-body";
const GUTTER_GAP: &str = "  ";
const ROWS_ABOVE_MATCH: f32 = 3.0;
const SELECT_ALL: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::A);

struct RowStyle {
    format: ResponseFormat,
    font: FontId,
    height: f32,
    palette: SyntaxPalette,
    gutter_digits: usize,
    gutter_color: Color32,
    text_color: Color32,
}

struct LaidRow {
    gutter: Arc<Galley>,
    text: Arc<Galley>,
}

impl RowStyle {
    fn new(ui: &Ui, buffer: &ResponseBuffer, format: ResponseFormat) -> Self {
        let visuals = ui.visuals();
        RowStyle {
            format,
            font: TextStyle::Monospace.resolve(ui.style()),
            height: ui.text_style_height(&TextStyle::Monospace),
            palette: SyntaxPalette::for_mode(visuals.dark_mode),
            gutter_digits: digits(buffer.row_count()),
            gutter_color: visuals.weak_text_color(),
            text_color: visuals.text_color(),
        }
    }

    fn lay_out(&self, ui: &Ui, buffer: &ResponseBuffer, index: usize) -> LaidRow {
        let width = self.gutter_digits;
        let gutter = LayoutJob::simple_singleline(
            format!("{:>width$}{GUTTER_GAP}", index + 1),
            self.font.clone(),
            self.gutter_color,
        );
        let mut text = LayoutJob::default();
        append_row(
            &mut text,
            buffer.row(index),
            self.format,
            &self.palette,
            &self.font,
        );
        LaidRow {
            gutter: ui.painter().layout_job(gutter),
            text: ui.painter().layout_job(text),
        }
    }
}

pub(super) fn show(ui: &mut Ui, display: BodyDisplay<'_>, format: ResponseFormat) {
    let BodyDisplay {
        buffer,
        search,
        selection,
    } = display;
    let style = RowStyle::new(ui, buffer, format);
    let mut area = ScrollArea::both().id_salt(SCROLL_ID).auto_shrink(false);
    if let Some(target) = search.take_scroll_target() {
        let spaced = style.height + ui.spacing().item_spacing.y;
        area = area.vertical_scroll_offset((target as f32 - ROWS_ABOVE_MATCH).max(0.0) * spaced);
    }
    let output = area.show_rows(ui, style.height, buffer.row_count(), |ui, visible| {
        for index in visible.clone() {
            if search.is_match(index) {
                mark_row(ui, style.height, search.current_row() == Some(index));
            }
            show_row(ui, &style, buffer, selection, index);
        }
        scroll_while_dragging(ui, style.height, buffer, selection, visible);
    });
    let hovered = ui.rect_contains_pointer(output.inner_rect);
    handle_keys(ui, buffer, selection, hovered);
}

fn show_row(
    ui: &mut Ui,
    style: &RowStyle,
    buffer: &ResponseBuffer,
    selection: &mut BodySelection,
    index: usize,
) {
    let row = style.lay_out(ui, buffer, index);
    let content = row.gutter.size().x + row.text.size().x;
    let size = vec2(content.max(ui.available_width()), style.height);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let origin = rect.min + vec2(row.gutter.size().x, 0.0);
    if let Some(columns) = selection.columns_in_row(buffer, index) {
        paint_selection(ui, &row.text, origin, columns, style.height);
    }
    ui.painter()
        .galley(rect.min, Arc::clone(&row.gutter), style.text_color);
    ui.painter()
        .galley(origin, Arc::clone(&row.text), style.text_color);
    let offset_at =
        |pos: Pos2| buffer.offset_at(index, row.text.cursor_from_pos(pos - origin).index.0);
    follow_pointer(ui, &response, selection, offset_at);
}

fn follow_pointer(
    ui: &Ui,
    response: &Response,
    selection: &mut BodySelection,
    offset_at: impl Fn(Pos2) -> usize,
) {
    if response.drag_started() || response.clicked() {
        ui.memory_mut(|memory| memory.stop_text_input());
    }
    if response.drag_started() {
        if let Some(press) = ui.input(|input| input.pointer.press_origin()) {
            selection.begin(offset_at(press));
        }
    } else if response.clicked() {
        selection.clear();
    }
    let pointer = ui.input(|input| input.pointer.latest_pos());
    if let Some(pos) = pointer.filter(|pos| response.rect.y_range().contains(pos.y)) {
        selection.extend(offset_at(pos));
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Text);
    }
}

fn paint_selection(ui: &Ui, text: &Galley, origin: Pos2, columns: Range<usize>, height: f32) {
    let start = text.pos_from_cursor(CCursor::new(columns.start)).min.x;
    let end = text.pos_from_cursor(CCursor::new(columns.end)).min.x;
    let rect = Rect::from_min_max(origin + vec2(start, 0.0), origin + vec2(end, height));
    ui.painter()
        .rect_filled(rect, 0.0, ui.visuals().selection.bg_fill);
}

fn scroll_while_dragging(
    ui: &Ui,
    row_height: f32,
    buffer: &ResponseBuffer,
    selection: &mut BodySelection,
    visible: Range<usize>,
) {
    let pointer = ui.input(|input| input.pointer.latest_pos());
    let (Some(pos), true, false) = (pointer, selection.is_dragging(), visible.is_empty()) else {
        return;
    };
    let clip = ui.clip_rect();
    if pos.y < clip.top() {
        ui.scroll_with_delta(vec2(0.0, row_height));
        selection.extend(buffer.row_span(visible.start).start);
    } else if pos.y > clip.bottom() {
        ui.scroll_with_delta(vec2(0.0, -row_height));
        selection.extend(buffer.row_span(visible.end - 1).end);
    } else {
        return;
    }
    ui.ctx().request_repaint();
}

fn handle_keys(ui: &Ui, buffer: &ResponseBuffer, selection: &mut BodySelection, hovered: bool) {
    if !ui.input(|input| input.pointer.primary_down()) {
        selection.finish();
    }
    if ui.memory(|memory| memory.focused().is_some()) {
        return;
    }
    if hovered && ui.input_mut(|input| input.consume_shortcut(&SELECT_ALL)) {
        selection.select_all(buffer);
    }
    let copy = ui.input(|input| input.events.contains(&Event::Copy));
    if let Some(text) = selection.text(buffer).filter(|_| copy) {
        ui.ctx().copy_text(text.to_string());
    }
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
