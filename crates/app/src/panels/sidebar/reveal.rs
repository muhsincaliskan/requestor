use eframe::egui::collapsing_header::CollapsingState;
use eframe::egui::{Id, Ui};
use requestor_core::collection::Collection;

use super::tree::{COLLECTION_NODE_ID, FOLDER_NODE_ID};
use crate::state::{AppState, ItemBinding};

const REVEALED_ID: &str = "sidebar-revealed";

pub(super) fn active_item(ui: &Ui, state: &AppState) -> bool {
    let Some(binding) = state.active().and_then(|tab| tab.binding.as_ref()) else {
        return false;
    };
    let key = Id::new(REVEALED_ID);
    if ui.data(|data| data.get_temp::<ItemBinding>(key)).as_ref() == Some(binding) {
        return false;
    }
    ui.data_mut(|data| data.insert_temp(key, binding.clone()));
    let collection = state
        .workspace
        .collections
        .iter()
        .find(|collection| collection.id() == binding.collection_id);
    collection.is_some_and(|collection| open_path(ui, collection, &binding.item_id))
}

fn open_path(ui: &Ui, collection: &Collection, item_id: &str) -> bool {
    let Some(folders) = collection.ancestors_of(item_id) else {
        return false;
    };
    open(ui, Id::new((COLLECTION_NODE_ID, collection.id())));
    for folder_id in folders.iter().filter_map(|folder| folder.id.as_deref()) {
        open(ui, Id::new((FOLDER_NODE_ID, collection.id(), folder_id)));
    }
    true
}

fn open(ui: &Ui, id: Id) {
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, false);
    state.set_open(true);
    state.store(ui.ctx());
}
