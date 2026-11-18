use eframe::egui::{Color32, Label, RichText, ScrollArea, TextStyle, Ui};
use requestor_core::text_diff::DiffRow;

use crate::response::{ChangesUnavailable, ResponseView};
use crate::widgets::success_color;

const SCROLL_ID: &str = "response-changes";
const BINARY_TEXT: &str = "Binary responses cannot be compared";
const FOLDED_TEXT: &str = "unchanged lines";
const FOLDED_MARK: &str = "⋯";
const UNCHANGED_TEXT: &str = "Same as the previous response";
const TOO_LARGE_TEXT: &str = "Too large or too different to compare with the previous response";
const SUMMARY_HINT: &str = "Compared with the previous response in this tab";
const ADDED_SIGN: &str = "+";
const REMOVED_SIGN: &str = "−";
const MARK_GAP: &str = " ";
const KEPT_MARK: &str = "  ";
const ROW_TINT: f32 = 0.12;

pub fn show(ui: &mut Ui, view: &mut ResponseView) {
    let diff = match view.changes() {
        Ok(diff) => diff,
        Err(reason) => {
            ui.weak(unavailable_text(reason));
            return;
        }
    };
    if diff.is_unchanged() {
        ui.weak(UNCHANGED_TEXT);
        return;
    }
    let added = RichText::new(format!("{ADDED_SIGN}{}", diff.added)).color(success_color(ui));
    let removed =
        RichText::new(format!("{REMOVED_SIGN}{}", diff.removed)).color(ui.visuals().error_fg_color);
    ui.horizontal(|ui| {
        ui.label(added).on_hover_text(SUMMARY_HINT);
        ui.label(removed).on_hover_text(SUMMARY_HINT);
    });
    ui.separator();
    let row_height = ui.text_style_height(&TextStyle::Monospace);
    ScrollArea::both()
        .id_salt(SCROLL_ID)
        .auto_shrink(false)
        .show_rows(ui, row_height, diff.rows.len(), |ui, range| {
            for row in &diff.rows[range] {
                ui.add(Label::new(row_text(ui, row)).extend());
            }
        });
}

fn row_text(ui: &Ui, row: &DiffRow) -> RichText {
    let text = match row {
        DiffRow::Kept(line) => RichText::new(format!("{KEPT_MARK}{line}")),
        DiffRow::Added(line) => tinted(format!("{ADDED_SIGN}{MARK_GAP}{line}"), success_color(ui)),
        DiffRow::Removed(line) => tinted(
            format!("{REMOVED_SIGN}{MARK_GAP}{line}"),
            ui.visuals().error_fg_color,
        ),
        DiffRow::Folded(count) => {
            RichText::new(format!("{FOLDED_MARK} {count} {FOLDED_TEXT}")).weak()
        }
    };
    text.monospace()
}

fn tinted(text: String, color: Color32) -> RichText {
    RichText::new(text)
        .color(color)
        .background_color(color.gamma_multiply(ROW_TINT))
}

fn unavailable_text(reason: ChangesUnavailable) -> &'static str {
    match reason {
        ChangesUnavailable::Binary => BINARY_TEXT,
        ChangesUnavailable::TooLarge => TOO_LARGE_TEXT,
    }
}
