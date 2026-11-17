use eframe::egui::{Grid, ScrollArea, TextEdit, Ui};

pub(super) const ADD_VARIABLE_LABEL: &str = "Add variable";
pub(super) const KEY_HINT: &str = "Variable";
pub(super) const VALUE_HINT: &str = "Value";
pub(super) const DONE_LABEL: &str = "Done";
pub(super) const NAME_LABEL: &str = "Name";
const DELETE_LABEL: &str = "Delete";
const TABLE_HEIGHT: f32 = 320.0;
const CELL_WIDTH: f32 = 200.0;

pub(super) enum RowEdit {
    Keep,
    Changed,
    Delete,
}

impl RowEdit {
    pub(super) fn from_flags(changed: bool, delete: bool) -> Self {
        if delete {
            RowEdit::Delete
        } else if changed {
            RowEdit::Changed
        } else {
            RowEdit::Keep
        }
    }
}

pub(super) fn edit_rows<T>(
    ui: &mut Ui,
    id: &str,
    columns: usize,
    rows: &mut Vec<T>,
    mut edit: impl FnMut(&mut Ui, &mut T) -> RowEdit,
) -> bool {
    let mut changed = false;
    let mut removed = None;
    ScrollArea::vertical()
        .id_salt(id)
        .max_height(TABLE_HEIGHT)
        .show(ui, |ui| {
            Grid::new(id)
                .num_columns(columns)
                .striped(true)
                .show(ui, |ui| {
                    for (index, row) in rows.iter_mut().enumerate() {
                        match edit(ui, row) {
                            RowEdit::Keep => {}
                            RowEdit::Changed => changed = true,
                            RowEdit::Delete => removed = Some(index),
                        }
                        ui.end_row();
                    }
                });
        });
    if let Some(index) = removed {
        rows.remove(index);
        changed = true;
    }
    changed
}

pub(super) fn text_cell(ui: &mut Ui, text: &mut String, hint: &str, masked: bool) -> bool {
    let edit = TextEdit::singleline(text)
        .hint_text(hint)
        .password(masked)
        .desired_width(CELL_WIDTH);
    let size = [CELL_WIDTH, ui.spacing().interact_size.y];
    ui.add_sized(size, edit).changed()
}

pub(super) fn enabled_cell(ui: &mut Ui, enabled: bool) -> Option<bool> {
    let mut checked = enabled;
    ui.checkbox(&mut checked, "").changed().then_some(checked)
}

pub(super) fn delete_cell(ui: &mut Ui) -> bool {
    ui.button(DELETE_LABEL).clicked()
}
