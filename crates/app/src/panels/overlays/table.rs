use eframe::egui::{vec2, Align, Button, Context, Grid, Id, Layout, ScrollArea, TextEdit, Ui};

use crate::widgets::variable_text_edit::FIELD_MARGIN;
use crate::widgets::{action_button, icon_button};

const ADD_VARIABLE_LABEL: &str = "+ Add variable";
pub(super) const KEY_HINT: &str = "Variable";
pub(super) const VALUE_HINT: &str = "Value";
const DONE_LABEL: &str = "Done";
const REVEAL_LABEL: &str = "Show secret values";
const NAME_LABEL: &str = "Name";
const DELETE_LABEL: &str = "×";
const DELETE_HINT: &str = "Delete variable";
const TABLE_HEIGHT: f32 = 320.0;
const CELL_WIDTH: f32 = 220.0;
const NAME_WIDTH: f32 = 320.0;
const FOOTER_GAP: f32 = 8.0;

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

pub(super) fn name_field(ui: &mut Ui, name: &mut String) -> bool {
    ui.horizontal(|ui| {
        ui.label(NAME_LABEL);
        let edit = TextEdit::singleline(name)
            .margin(FIELD_MARGIN)
            .desired_width(NAME_WIDTH);
        ui.add(edit).changed()
    })
    .inner
}

pub(super) fn text_cell(ui: &mut Ui, text: &mut String, hint: &str, masked: bool) -> bool {
    let edit = TextEdit::singleline(text)
        .hint_text(hint)
        .password(masked)
        .margin(FIELD_MARGIN)
        .desired_width(CELL_WIDTH)
        .min_size(vec2(CELL_WIDTH, 0.0));
    ui.add(edit).changed()
}

pub(super) fn enabled_cell(ui: &mut Ui, enabled: bool) -> Option<bool> {
    let mut checked = enabled;
    ui.checkbox(&mut checked, "").changed().then_some(checked)
}

pub(super) fn delete_cell(ui: &mut Ui) -> bool {
    icon_button(ui, DELETE_LABEL, DELETE_HINT).clicked()
}

pub(super) fn footer(ui: &mut Ui) -> (bool, bool) {
    let width = ui.min_rect().width();
    ui.add_space(FOOTER_GAP);
    let size = vec2(width, ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
        let add = ui
            .add(Button::new(ADD_VARIABLE_LABEL).frame(false))
            .clicked();
        let done = ui
            .with_layout(Layout::right_to_left(Align::Center), |ui| {
                action_button::primary(ui, DONE_LABEL).clicked()
            })
            .inner;
        (add, done)
    })
    .inner
}

pub(super) fn reveal_toggle(ui: &mut Ui, id: &str) -> bool {
    let id = Id::new(id);
    let mut reveal = ui.data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
    if ui.checkbox(&mut reveal, REVEAL_LABEL).changed() {
        ui.data_mut(|data| data.insert_temp(id, reveal));
    }
    reveal
}

pub(super) fn forget_reveal(ctx: &Context, id: &str) {
    ctx.data_mut(|data| data.remove_temp::<bool>(Id::new(id)));
}

pub(super) fn footer_done(ui: &mut Ui) -> bool {
    ui.add_space(FOOTER_GAP);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        action_button::primary(ui, DONE_LABEL).clicked()
    })
    .inner
}
