use eframe::egui::{ScrollArea, Ui};

use crate::state::{Action, AppState, RequestTab};
use crate::widgets::{icon_button, request_title};

const STRIP_SCROLL_ID: &str = "tab-strip-scroll";
const NEW_TAB_LABEL: &str = "+";
const NEW_TAB_HINT: &str = "New tab";
const CLOSE_LABEL: &str = "×";
const CLOSE_HINT: &str = "Close tab";
const UNSAVED_MARK: &str = " •";
const SAVE_ITEM: &str = "Save";
const DUPLICATE_ITEM: &str = "Duplicate";
const CLOSE_ITEM: &str = "Close";
const CLOSE_OTHERS_ITEM: &str = "Close others";

pub fn show(ui: &mut Ui, state: &AppState, actions: &mut Vec<Action>) {
    let with_method = state.interface().show_icons_with_tab_names;
    ScrollArea::horizontal()
        .id_salt(STRIP_SCROLL_ID)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for tab in &state.tabs {
                    let active = state.active_tab == Some(tab.id);
                    ui.push_id(tab.id, |ui| {
                        tab_button(ui, tab, active, with_method, actions)
                    });
                }
                if ui
                    .button(NEW_TAB_LABEL)
                    .on_hover_text(NEW_TAB_HINT)
                    .clicked()
                {
                    actions.push(Action::NewTab);
                }
            });
        });
}

fn tab_button(
    ui: &mut Ui,
    tab: &RequestTab,
    active: bool,
    with_method: bool,
    actions: &mut Vec<Action>,
) {
    let mark = if tab.unsaved { UNSAVED_MARK } else { "" };
    let name = format!("{}{mark}", tab.draft.name);
    let method = with_method.then_some(tab.draft.method.as_str());
    let button = ui.selectable_label(active, request_title(ui, method, &name));
    if button.clicked() {
        actions.push(Action::ActivateTab(tab.id));
    }
    if button.middle_clicked() || icon_button(ui, CLOSE_LABEL, CLOSE_HINT).clicked() {
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
