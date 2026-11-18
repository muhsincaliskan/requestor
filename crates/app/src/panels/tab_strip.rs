use eframe::egui::scroll_area::{ScrollAreaOutput, ScrollBarVisibility};
use eframe::egui::{
    vec2, Atom, Button, Direction, Id, Layout, Rect, Response, ScrollArea, Ui, UiBuilder,
};

mod overflow;

use crate::state::{Action, AppState, RequestTab};
use crate::widgets::{close_button, request_title, ICON_BUTTON_SIZE, UNSAVED_DOT};

const STRIP_SCROLL_ID: &str = "tab-strip-scroll";
const NEW_TAB_LABEL: &str = "+";
const NEW_TAB_HINT: &str = "New tab";
const CLOSE_SLOT_ID: &str = "tab-close-slot";
const CLOSE_HINT: &str = "Close tab";
const UNSAVED_HINT: &str = "Unsaved changes";
const SAVE_ITEM: &str = "Save";
const DUPLICATE_ITEM: &str = "Duplicate";
const CLOSE_ITEM: &str = "Close";
const CLOSE_OTHERS_ITEM: &str = "Close others";

pub fn show(ui: &mut Ui, state: &AppState, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        let controls = 2.0 * (ICON_BUTTON_SIZE + ui.spacing().item_spacing.x);
        let strip = vec2(
            (ui.available_width() - controls).max(0.0),
            ui.available_height(),
        );
        let output = ui.allocate_ui(strip, |ui| tabs(ui, state, actions)).inner;
        overflow::fade_edges(ui, &output);
        if overflow::overflows(&output) {
            overflow::all_tabs_menu(ui, state, actions);
        }
        if ui
            .button(NEW_TAB_LABEL)
            .on_hover_text(NEW_TAB_HINT)
            .clicked()
        {
            actions.push(Action::NewTab);
        }
    });
}

fn tabs(ui: &mut Ui, state: &AppState, actions: &mut Vec<Action>) -> ScrollAreaOutput<()> {
    let with_method = state.interface().show_icons_with_tab_names;
    ScrollArea::horizontal()
        .id_salt(STRIP_SCROLL_ID)
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for tab in &state.tabs {
                    let active = state.active_tab == Some(tab.id);
                    ui.push_id(tab.id, |ui| {
                        tab_button(ui, tab, active, with_method, actions)
                    });
                }
            });
            overflow::scroll_wheel_sideways(ui);
        })
}

fn tab_button(
    ui: &mut Ui,
    tab: &RequestTab,
    active: bool,
    with_method: bool,
    actions: &mut Vec<Action>,
) {
    let method = with_method.then_some(tab.draft.method.as_str());
    let slot_id = Id::new(CLOSE_SLOT_ID);
    let slot = Atom::custom(slot_id, vec2(ICON_BUTTON_SIZE, ICON_BUTTON_SIZE));
    let laid_out =
        Button::selectable(active, (request_title(ui, method, &tab.draft.name), slot)).atom_ui(ui);
    let button = laid_out.response.clone();
    if button.clicked() {
        actions.push(Action::ActivateTab(tab.id));
    }
    if active {
        overflow::reveal_when_activated(ui, &button, tab.id);
    }
    let closed = laid_out
        .rect(slot_id)
        .is_some_and(|slot| close_slot(ui, &button, slot, tab.unsaved, active));
    if button.middle_clicked() || closed {
        actions.push(Action::CloseTab(tab.id));
    }
    button.context_menu(|ui| {
        let items = [
            (SAVE_ITEM, Action::SaveTab(tab.id)),
            (DUPLICATE_ITEM, Action::DuplicateTab(tab.id)),
            (CLOSE_ITEM, Action::CloseTab(tab.id)),
            (CLOSE_OTHERS_ITEM, Action::CloseOtherTabs(tab.id)),
        ];
        for (label, action) in items {
            if ui.button(label).clicked() {
                actions.push(action);
                ui.close();
            }
        }
    });
}

fn close_slot(ui: &mut Ui, button: &Response, slot: Rect, unsaved: bool, active: bool) -> bool {
    let hovered = ui.rect_contains_pointer(button.rect);
    let shows_close = hovered || (active && !unsaved);
    let mut slot_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(slot)
            .layout(Layout::centered_and_justified(Direction::LeftToRight)),
    );
    if shows_close {
        return close_button(&mut slot_ui, CLOSE_HINT).clicked();
    }
    if unsaved {
        slot_ui.weak(UNSAVED_DOT).on_hover_text(UNSAVED_HINT);
    }
    false
}
