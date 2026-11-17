use requestor_core::collection::{Collection, Item, Request, Url};
use requestor_core::method::HttpMethod;

use super::overlay::{LibraryTarget, Overlay, RenameForm};
use super::tab::UNTITLED_REQUEST;
use super::{Action, AppState, Effect};

const NEW_COLLECTION_NAME: &str = "New Collection";
const NEW_FOLDER_NAME: &str = "New Folder";

impl AppState {
    pub(super) fn dispatch_library_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::NewCollection => self.new_collection(),
            Action::AddFolder {
                collection_id,
                parent_id,
            } => self.add_folder(&collection_id, parent_id.as_deref()),
            Action::AddRequest {
                collection_id,
                parent_id,
            } => self.add_request(&collection_id, parent_id.as_deref()),
            Action::DuplicateItem {
                collection_id,
                item_id,
            } => self.duplicate_item(&collection_id, &item_id),
            Action::MoveItem {
                collection_id,
                item_id,
                parent_id,
                index,
            } => self.move_item(&collection_id, &item_id, parent_id.as_deref(), index),
            Action::Rename { target, name } => self.rename(&target, name),
            Action::Delete(target) => self.delete(&target, effects),
            Action::CollectionEdited(id) => self.unsaved.touch_collection(&id, self.now),
            other => self.dispatch_environment_action(other, effects),
        }
    }

    fn new_collection(&mut self) {
        let id = self
            .workspace
            .add_collection(Collection::new(NEW_COLLECTION_NAME));
        self.unsaved.touch_collection(&id, self.now);
        self.overlay = Some(Overlay::Rename(RenameForm {
            target: LibraryTarget::Collection { collection_id: id },
            name: NEW_COLLECTION_NAME.to_string(),
        }));
    }

    fn add_folder(&mut self, collection_id: &str, parent_id: Option<&str>) {
        let folder = Item::folder(NEW_FOLDER_NAME.to_string(), Vec::new());
        let Some(item_id) = self.add_item(collection_id, parent_id, folder) else {
            return;
        };
        self.overlay = Some(Overlay::Rename(RenameForm {
            target: LibraryTarget::Item {
                collection_id: collection_id.to_string(),
                item_id,
            },
            name: NEW_FOLDER_NAME.to_string(),
        }));
    }

    fn add_request(&mut self, collection_id: &str, parent_id: Option<&str>) {
        let request = Request::new(HttpMethod::Get.as_str(), Url::Raw(String::new()));
        let item = Item::for_request(UNTITLED_REQUEST.to_string(), request);
        if let Some(item_id) = self.add_item(collection_id, parent_id, item) {
            self.open_item(collection_id, &item_id);
        }
    }

    fn add_item(
        &mut self,
        collection_id: &str,
        parent_id: Option<&str>,
        item: Item,
    ) -> Option<String> {
        let created = self
            .workspace
            .collection_mut(collection_id)?
            .add_item(parent_id, item);
        match &created {
            Some(_) => self.unsaved.touch_collection(collection_id, self.now),
            None => self.report_error("Items can only be added to a collection or a folder"),
        }
        created
    }

    fn duplicate_item(&mut self, collection_id: &str, item_id: &str) {
        let duplicated = self
            .workspace
            .collection_mut(collection_id)
            .and_then(|collection| collection.duplicate_item(item_id));
        if duplicated.is_some() {
            self.unsaved.touch_collection(collection_id, self.now);
        }
    }

    fn move_item(
        &mut self,
        collection_id: &str,
        item_id: &str,
        parent_id: Option<&str>,
        index: usize,
    ) {
        let moved = self
            .workspace
            .collection_mut(collection_id)
            .is_some_and(|collection| collection.move_item(item_id, parent_id, index));
        if moved {
            self.unsaved.touch_collection(collection_id, self.now);
        }
    }

    fn rename(&mut self, target: &LibraryTarget, name: String) {
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        match target {
            LibraryTarget::Collection { collection_id } => {
                if let Some(collection) = self.workspace.collection_mut(collection_id) {
                    collection.info.name = name;
                    self.unsaved.touch_collection(collection_id, self.now);
                }
            }
            LibraryTarget::Item {
                collection_id,
                item_id,
            } => self.rename_item(collection_id, item_id, name),
            LibraryTarget::Environment { environment_id } => {
                if let Some(environment) = self.workspace.environment_mut(environment_id) {
                    environment.name = name;
                    self.unsaved.touch_environment(environment_id, self.now);
                }
            }
        }
        self.overlay = None;
    }

    fn rename_item(&mut self, collection_id: &str, item_id: &str, name: String) {
        let Some(item) = self
            .workspace
            .collection_mut(collection_id)
            .and_then(|collection| collection.find_item_mut(item_id))
        else {
            return;
        };
        item.name = Some(name.clone());
        self.unsaved.touch_collection(collection_id, self.now);
        for tab in &mut self.tabs {
            if tab.is_bound_to(collection_id, item_id) {
                tab.draft.name.clone_from(&name);
            }
        }
        self.unsaved.touch_session(self.now);
    }

    fn delete(&mut self, target: &LibraryTarget, effects: &mut Vec<Effect>) {
        match target {
            LibraryTarget::Collection { collection_id } => {
                if self.workspace.remove_collection(collection_id).is_some() {
                    self.unsaved.forget_collection(collection_id);
                    effects.extend(
                        self.store
                            .release_collection_path(collection_id)
                            .map(Effect::DeleteFile),
                    );
                }
            }
            LibraryTarget::Item {
                collection_id,
                item_id,
            } => {
                let removed = self
                    .workspace
                    .collection_mut(collection_id)
                    .and_then(|collection| collection.remove_item(item_id));
                if removed.is_some() {
                    self.unsaved.touch_collection(collection_id, self.now);
                }
            }
            LibraryTarget::Environment { environment_id } => {
                if self.workspace.remove_environment(environment_id).is_some() {
                    self.unsaved.forget_environment(environment_id);
                    effects.extend(
                        self.store
                            .release_environment_path(environment_id)
                            .map(Effect::DeleteFile),
                    );
                }
            }
        }
        self.unbind_orphaned_tabs();
        self.overlay = None;
    }

    fn unbind_orphaned_tabs(&mut self) {
        let workspace = &self.workspace;
        for tab in &mut self.tabs {
            let orphaned = tab.binding.as_ref().is_some_and(|bound| {
                workspace
                    .collection(&bound.collection_id)
                    .and_then(|collection| collection.find_item(&bound.item_id))
                    .is_none()
            });
            if orphaned {
                tab.binding = None;
                tab.unsaved = true;
            }
        }
        self.unsaved.touch_session(self.now);
    }
}
