use std::path::Path;

use eframe::egui::collapsing_header::CollapsingState;
use eframe::egui::{Align, Button, Id, Label, Response, RichText, Sense, Ui, WidgetText};
use requestor_core::collection::{Collection, Item};

use super::drag::{self, DropSlot};
use super::{menus, reveal};
use crate::state::{Action, AppState, ItemBinding};
use crate::widgets::{aligned_request_title, list_row, row_label, UNSAVED_MARK};

pub(super) const COLLECTION_NODE_ID: &str = "sidebar-collection";
pub(super) const FOLDER_NODE_ID: &str = "sidebar-folder";
const NO_MATCH_TEXT: &str = "No requests match the filter.";
const SAVED_IN_TEXT: &str = "Saved in";

struct TreeView<'a> {
    collection: &'a Collection,
    folder: Option<&'a Path>,
    active: Option<&'a ItemBinding>,
    awaiting_save: &'a [&'a ItemBinding],
    filter: &'a str,
    revealed: bool,
    actions: &'a mut Vec<Action>,
}

pub(super) fn show(ui: &mut Ui, state: &AppState, filter: &str, actions: &mut Vec<Action>) {
    let active = state.active().and_then(|tab| tab.binding.as_ref());
    let revealed = reveal::active_item(ui, state);
    let awaiting_save: Vec<&ItemBinding> = state
        .tabs_awaiting_save()
        .filter_map(|tab| tab.binding.as_ref())
        .collect();
    let mut shown_any = false;
    for collection in &state.workspace.collections {
        if collection.matches_filter(filter) {
            shown_any = true;
            let mut view = TreeView {
                collection,
                folder: state.linked_folder(collection.id()),
                active,
                awaiting_save: &awaiting_save,
                filter,
                revealed,
                actions: &mut *actions,
            };
            collection_node(ui, &mut view);
        }
    }
    if !shown_any {
        ui.weak(NO_MATCH_TEXT);
    }
}

pub(super) fn branch(
    ui: &mut Ui,
    id: Id,
    force_open: bool,
    header: impl FnOnce(&mut Ui) -> Response,
    body: impl FnOnce(&mut Ui),
) -> Response {
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, false);
    if force_open {
        state.set_open(true);
    }
    let mut header_clicked = false;
    let mut shown = state.show_header(ui, |ui| {
        let response = header(ui);
        header_clicked = response.clicked();
        response
    });
    if header_clicked {
        shown.toggle();
    }
    let (_, header, _) = shown.body(body);
    header.inner
}

fn collection_node(ui: &mut Ui, view: &mut TreeView) {
    let collection = view.collection;
    let folder = view.folder;
    let id = Id::new((COLLECTION_NODE_ID, collection.id()));
    let response = branch(
        ui,
        id,
        !view.filter.is_empty(),
        |ui| {
            let name = RichText::new(&collection.info.name).strong();
            let label = ui.add(Label::new(name).sense(Sense::click()));
            match folder {
                Some(folder) => {
                    label.on_hover_text(format!("{SAVED_IN_TEXT} {}", folder.display()))
                }
                None => label,
            }
        },
        |ui| items(ui, view, &collection.item),
    );
    menus::collection(&response, collection, folder, view.actions);
    drag::accept(
        ui,
        &response,
        collection,
        DropSlot::CollectionRoot,
        view.actions,
    );
}

fn items(ui: &mut Ui, view: &mut TreeView, items: &[Item]) {
    for item in items {
        if view.filter.is_empty() || item.matches_filter(view.filter) {
            item_node(ui, view, item);
        }
    }
}

fn item_node(ui: &mut Ui, view: &mut TreeView, item: &Item) {
    if item.id.is_none() {
        return;
    }
    if item.is_folder() {
        folder_node(ui, view, item);
    } else {
        request_node(ui, view, item);
    }
}

fn folder_node(ui: &mut Ui, view: &mut TreeView, item: &Item) {
    let Some(item_id) = item.id.as_deref() else {
        return;
    };
    let collection = view.collection;
    let id = Id::new((FOLDER_NODE_ID, collection.id(), item_id));
    let response = branch(
        ui,
        id,
        !view.filter.is_empty(),
        |ui| {
            let name = WidgetText::from(item.display_name());
            ui.add(
                Button::new(name)
                    .frame(false)
                    .sense(Sense::click_and_drag()),
            )
        },
        |ui| items(ui, view, item.children()),
    );
    drag::begin(&response, collection.id(), item_id);
    drag::accept(
        ui,
        &response,
        collection,
        DropSlot::IntoFolder(item_id),
        view.actions,
    );
    menus::item(&response, collection.id(), item, view.actions);
}

fn request_node(ui: &mut Ui, view: &mut TreeView, item: &Item) {
    let Some(item_id) = item.id.as_deref() else {
        return;
    };
    let collection = view.collection;
    let selected = view
        .active
        .is_some_and(|binding| binding.points_at(collection.id(), item_id));
    let unsaved = view
        .awaiting_save
        .iter()
        .any(|binding| binding.points_at(collection.id(), item_id));
    let mark = if unsaved { UNSAVED_MARK } else { "" };
    let name = format!("{}{mark}", item.display_name());
    let title = aligned_request_title(ui, item.method_name(), &name);
    let response = ui.add(list_row(ui, selected, row_label(title)).sense(Sense::click_and_drag()));
    if selected && view.revealed {
        response.scroll_to_me(Some(Align::Center));
    }
    if response.clicked() {
        view.actions.push(Action::OpenItem {
            collection_id: collection.id().to_string(),
            item_id: item_id.to_string(),
        });
    }
    drag::begin(&response, collection.id(), item_id);
    drag::accept(
        ui,
        &response,
        collection,
        DropSlot::Beside(item_id),
        view.actions,
    );
    menus::item(&response, collection.id(), item, view.actions);
}
