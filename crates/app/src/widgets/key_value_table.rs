use eframe::egui::{Checkbox, Id, Label, RichText, Ui};
use requestor_core::draft::KeyValueRow;

use super::icon_button;
use super::variable_text_edit::{VariableHints, VariableTextEdit};

const KEY_HINT: &str = "Key";
const VALUE_HINT: &str = "Value";
const FILE_HINT: &str = "File path";
const DESCRIPTION_HINT: &str = "Description";
const DELETE_LABEL: &str = "×";
const DELETE_HINT: &str = "Delete row";
const TEXT_KIND_LABEL: &str = "Text";
const FILE_KIND_LABEL: &str = "File";
const GRID_SALT: &str = "grid";
const PENDING_SALT: &str = "pending";
const CHECKBOX_COLUMN_WIDTH: f32 = 22.0;
const DELETE_COLUMN_WIDTH: f32 = 26.0;
const KIND_BUTTON_WIDTH: f32 = 44.0;
const COLUMN_SPACING: f32 = 6.0;
const COLUMN_COUNT: usize = 5;
const MIN_TEXT_WIDTH: f32 = 90.0;
const KEY_SHARE: f32 = 0.28;
const VALUE_SHARE: f32 = 0.42;
const DESCRIPTION_SHARE: f32 = 0.30;

pub struct KeyValueTable<'a> {
    rows: &'a mut Vec<KeyValueRow>,
    hints: &'a VariableHints,
    id: Id,
    keys_editable: bool,
    rows_addable: bool,
    files_allowed: bool,
}

struct ColumnWidths {
    key: f32,
    value: f32,
    description: f32,
}

#[derive(Default)]
struct RowOutcome {
    changed: bool,
    delete: bool,
}

impl ColumnWidths {
    fn fit(available: f32) -> Self {
        let fixed =
            CHECKBOX_COLUMN_WIDTH + DELETE_COLUMN_WIDTH + COLUMN_SPACING * (COLUMN_COUNT as f32);
        let flexible = (available - fixed).max(MIN_TEXT_WIDTH * 3.0);
        ColumnWidths {
            key: flexible * KEY_SHARE,
            value: flexible * VALUE_SHARE,
            description: flexible * DESCRIPTION_SHARE,
        }
    }
}

impl<'a> KeyValueTable<'a> {
    pub fn new(rows: &'a mut Vec<KeyValueRow>, hints: &'a VariableHints, id_salt: &str) -> Self {
        KeyValueTable {
            rows,
            hints,
            id: Id::new(id_salt),
            keys_editable: true,
            rows_addable: true,
            files_allowed: false,
        }
    }

    pub fn fixed_keys(mut self) -> Self {
        self.keys_editable = false;
        self.rows_addable = false;
        self
    }

    pub fn with_files(mut self) -> Self {
        self.files_allowed = true;
        self
    }

    pub fn show(mut self, ui: &mut Ui) -> bool {
        self.id = ui.id().with(self.id);
        let widths = ColumnWidths::fit(ui.available_width());
        let mut changed = false;
        let mut removed = None;
        ui.push_id(self.id.with(GRID_SALT), |ui| {
            ui.spacing_mut().item_spacing.x = COLUMN_SPACING;
            changed |= self.show_rows(ui, &widths, &mut removed);
            changed |= self.show_pending(ui, &widths);
        });
        if let Some(index) = removed {
            self.rows.remove(index);
            changed = true;
        }
        changed
    }

    fn show_rows(
        &mut self,
        ui: &mut Ui,
        widths: &ColumnWidths,
        removed: &mut Option<usize>,
    ) -> bool {
        let mut changed = false;
        for index in 0..self.rows.len() {
            let outcome = self.show_row(ui, widths, index, None);
            changed |= outcome.changed;
            if outcome.delete {
                *removed = Some(index);
            }
        }
        changed
    }

    fn show_pending(&mut self, ui: &mut Ui, widths: &ColumnWidths) -> bool {
        if !self.rows_addable {
            return false;
        }
        let pending_id = self.id.with(PENDING_SALT);
        let pending: KeyValueRow = ui
            .data(|data| data.get_temp(pending_id))
            .unwrap_or_default();
        let mut pending = Some(pending);
        let index = self.rows.len();
        let outcome = self.show_row(ui, widths, index, pending.as_mut());
        let kept = match pending.take() {
            Some(row) if outcome.changed => row.commit_into(self.rows),
            other => other,
        };
        let committed = outcome.changed && kept.is_none();
        ui.data_mut(|data| data.insert_temp(pending_id, kept.unwrap_or_default()));
        committed
    }

    fn show_row(
        &mut self,
        ui: &mut Ui,
        widths: &ColumnWidths,
        index: usize,
        pending: Option<&mut KeyValueRow>,
    ) -> RowOutcome {
        let is_pending = pending.is_some();
        let hints = self.hints;
        let (keys_editable, files_allowed, id) = (self.keys_editable, self.files_allowed, self.id);
        let row = match pending {
            Some(row) => row,
            None => &mut self.rows[index],
        };
        let deletable = self.rows_addable && !is_pending;
        let mut outcome = RowOutcome::default();
        ui.horizontal(|ui| {
            outcome.changed |= ui.add(Checkbox::without_text(&mut row.enabled)).changed();
            outcome.changed |= key_cell(ui, row, widths.key, keys_editable, hints, id.with(index));
            outcome.changed |=
                value_cell(ui, row, widths.value, files_allowed, hints, id.with(index));
            outcome.changed |= description_cell(ui, row, widths.description, id.with(index));
            outcome.delete = deletable && icon_button(ui, DELETE_LABEL, DELETE_HINT).clicked();
        });
        outcome
    }
}

fn key_cell(
    ui: &mut Ui,
    row: &mut KeyValueRow,
    width: f32,
    editable: bool,
    hints: &VariableHints,
    id: Id,
) -> bool {
    if !editable {
        let size = [width, ui.spacing().interact_size.y];
        ui.add_sized(
            size,
            Label::new(RichText::new(&row.key).monospace()).truncate(),
        );
        return false;
    }
    VariableTextEdit::new(&mut row.key, hints)
        .hint_text(KEY_HINT)
        .desired_width(width)
        .id(id.with(KEY_HINT))
        .show(ui)
        .changed()
}

fn value_cell(
    ui: &mut Ui,
    row: &mut KeyValueRow,
    width: f32,
    files_allowed: bool,
    hints: &VariableHints,
    id: Id,
) -> bool {
    let mut changed = false;
    let mut edit_width = width;
    ui.horizontal(|ui| {
        if files_allowed {
            edit_width -= KIND_BUTTON_WIDTH;
            changed |= kind_toggle(ui, row);
        }
        let hint = if row.is_file { FILE_HINT } else { VALUE_HINT };
        changed |= VariableTextEdit::new(&mut row.value, hints)
            .hint_text(hint)
            .desired_width(edit_width)
            .id(id.with(VALUE_HINT))
            .show(ui)
            .changed();
    });
    changed
}

fn kind_toggle(ui: &mut Ui, row: &mut KeyValueRow) -> bool {
    let label = if row.is_file {
        FILE_KIND_LABEL
    } else {
        TEXT_KIND_LABEL
    };
    let clicked = ui.small_button(label).clicked();
    if clicked {
        row.is_file = !row.is_file;
    }
    clicked
}

fn description_cell(ui: &mut Ui, row: &mut KeyValueRow, width: f32, id: Id) -> bool {
    VariableTextEdit::plain(&mut row.description)
        .hint_text(DESCRIPTION_HINT)
        .desired_width(width)
        .id(id.with(DESCRIPTION_HINT))
        .show(ui)
        .changed()
}
