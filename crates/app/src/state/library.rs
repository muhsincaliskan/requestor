use std::path::Path;

use requestor_core::collection::{Collection, Item, Request, Url};
use requestor_core::environment::Environment;
use requestor_core::import::{import_text, Imported};
use requestor_core::method::HttpMethod;

use super::overlay::{LibraryTarget, Overlay, RenameForm};
use super::tab::{RequestTab, UNTITLED_REQUEST};
use super::{Action, AppState, Effect};

const NEW_COLLECTION_NAME: &str = "New Collection";
const NEW_FOLDER_NAME: &str = "New Folder";
const NEW_ENVIRONMENT_NAME: &str = "New Environment";
const MAX_IMPORT_FILE_BYTES: u64 = 64 * 1024 * 1024;

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

    fn dispatch_environment_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::NewEnvironment => self.new_environment(),
            Action::SelectEnvironment(id) => self.select_environment(id),
            Action::EnvironmentEdited(id) => self.unsaved.touch_environment(&id, self.now),
            Action::ImportText(text) => self.import(&text),
            Action::ImportFile(path) => self.import_file(&path),
            Action::Export { target, file_path } => self.export(&target, file_path, effects),
            _ => {}
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
                    effects.push(Effect::DeleteCollectionFile(collection_id.clone()));
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
                    effects.push(Effect::DeleteEnvironmentFile(environment_id.clone()));
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

    fn new_environment(&mut self) {
        let id = self
            .workspace
            .add_environment(Environment::new(NEW_ENVIRONMENT_NAME));
        self.unsaved.touch_environment(&id, self.now);
        self.overlay = Some(Overlay::EnvironmentEditor { environment_id: id });
    }

    fn select_environment(&mut self, id: Option<String>) {
        self.workspace.session.active_environment =
            id.filter(|id| self.workspace.environment(id).is_some());
        self.unsaved.touch_session(self.now);
    }

    fn import_file(&mut self, path: &Path) {
        let too_large =
            std::fs::metadata(path).is_ok_and(|file| file.len() > MAX_IMPORT_FILE_BYTES);
        if too_large {
            self.import_failed(format!("{} is too large to import", path.display()));
            return;
        }
        match std::fs::read_to_string(path) {
            Ok(text) => self.import(&text),
            Err(error) => self.import_failed(format!("Cannot read {}: {error}", path.display())),
        }
    }

    fn import(&mut self, text: &str) {
        let imported = match import_text(text) {
            Ok(imported) => imported,
            Err(error) => {
                self.import_failed(format!("Import failed: {error}"));
                return;
            }
        };
        for warning in imported.warnings().to_vec() {
            self.warn(warning);
        }
        match imported {
            Imported::Collection(imported) => {
                let name = imported.collection.info.name.clone();
                let id = self.workspace.add_collection(imported.collection);
                self.unsaved.touch_collection(&id, self.now);
                self.inform(format!("Imported collection {name}"));
            }
            Imported::Environment(environment) => {
                let name = environment.name.clone();
                let id = self.workspace.add_environment(environment);
                self.unsaved.touch_environment(&id, self.now);
                self.inform(format!("Imported environment {name}"));
            }
            Imported::Request(imported) => {
                let id = self.allocate_tab_id();
                self.open_tab(RequestTab::for_request(
                    id,
                    &imported.name,
                    &imported.request,
                ));
            }
        }
        self.overlay = None;
    }

    fn import_failed(&mut self, reason: String) {
        match &mut self.overlay {
            Some(Overlay::Import(form)) => form.error = Some(reason),
            _ => self.report_error(reason),
        }
    }

    fn export(
        &mut self,
        target: &LibraryTarget,
        file_path: std::path::PathBuf,
        effects: &mut Vec<Effect>,
    ) {
        let document = match target {
            LibraryTarget::Collection { collection_id } => self
                .workspace
                .collection(collection_id)
                .map(Collection::to_json_pretty),
            LibraryTarget::Item {
                collection_id,
                item_id,
            } => self
                .workspace
                .collection(collection_id)
                .and_then(|collection| collection.find_item(item_id))
                .map(|item| Collection::containing(item.clone()).to_json_pretty()),
            LibraryTarget::Environment { environment_id } => self
                .workspace
                .environment(environment_id)
                .map(Environment::to_json_pretty),
        };
        match document {
            Some(Ok(contents)) => {
                effects.push(Effect::WriteFile {
                    path: file_path,
                    contents,
                });
                self.overlay = None;
            }
            Some(Err(error)) => self.report_error(format!("Export failed: {error}")),
            None => self.report_error("Nothing to export"),
        }
    }
}
