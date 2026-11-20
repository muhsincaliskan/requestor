mod add_menu;
mod drag;
mod empty;
mod environments;
mod examples;
mod history;
mod menus;
mod reveal;
mod tree;
mod unsaved;

use eframe::egui::{vec2, Align, Id, Layout, ScrollArea, TextEdit, Ui};

use crate::state::{Action, AppState};
use crate::widgets::icon_button;
use crate::widgets::variable_text_edit::FIELD_MARGIN;

const TITLE: &str = "Collections";
const NEW_LABEL: &str = "+";
const NEW_COLLECTION_LABEL: &str = "New collection";
const CLEAR_LABEL: &str = "×";
const CLEAR_HINT: &str = "Clear filter";
const FILTER_HINT: &str = "Filter requests";
const FILTER_ID: &str = "sidebar-filter";
const FILTER_FIELD_KEY: &str = "field";
const SCROLL_ID: &str = "sidebar-scroll";
const SECTION_GAP: f32 = 8.0;
const ROW_GAP: f32 = 2.0;
const ROW_HEIGHT: f32 = 24.0;
const ROW_PADDING: (f32, f32) = (6.0, 3.0);

pub fn show(ui: &mut Ui, state: &mut AppState, actions: &mut Vec<Action>) {
    unsaved::show(ui, state, actions);
    header(ui, actions);
    let filter = if state.workspace.has_requests() {
        filter_field(ui)
    } else {
        String::new()
    };
    ui.add_space(SECTION_GAP);
    ScrollArea::vertical()
        .id_salt(SCROLL_ID)
        .auto_shrink(false)
        .show(ui, |ui| {
            compact_rows(ui);
            if state.workspace.collections.is_empty() {
                empty::show(ui, actions);
            } else {
                tree::show(ui, state, &filter, actions);
            }
            ui.add_space(SECTION_GAP);
            environments::show(ui, state, actions);
            ui.add_space(SECTION_GAP);
            history::show(ui, state, &filter, actions);
        });
}

fn header(ui: &mut Ui, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.strong(TITLE);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if icon_button(ui, NEW_LABEL, NEW_COLLECTION_LABEL).clicked() {
                actions.push(Action::NewCollection);
            }
            menus::import(ui, actions);
        });
    });
}

fn trailing_add_button(ui: &mut Ui, hint: &str) -> bool {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        icon_button(ui, NEW_LABEL, hint).clicked()
    })
    .inner
}

fn compact_rows(ui: &mut Ui) {
    let spacing = ui.spacing_mut();
    spacing.item_spacing.y = ROW_GAP;
    spacing.interact_size.y = ROW_HEIGHT;
    spacing.button_padding = vec2(ROW_PADDING.0, ROW_PADDING.1);
}

fn filter_field(ui: &mut Ui) -> String {
    let id = Id::new(FILTER_ID);
    let mut text: String = ui.data(|data| data.get_temp(id)).unwrap_or_default();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if !text.is_empty() && icon_button(ui, CLEAR_LABEL, CLEAR_HINT).clicked() {
                text.clear();
                changed = true;
            }
            let edit = TextEdit::singleline(&mut text)
                .id(id.with(FILTER_FIELD_KEY))
                .hint_text(FILTER_HINT)
                .margin(FIELD_MARGIN)
                .desired_width(f32::INFINITY);
            changed |= ui.add(edit).changed();
        });
    });
    if changed {
        ui.data_mut(|data| data.insert_temp(id, text.clone()));
    }
    text.trim().to_string()
}
