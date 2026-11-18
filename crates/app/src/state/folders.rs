use std::path::{Path, PathBuf};

use requestor_core::workspace::{StoreError, StoreWrite};

use super::{Action, AppState, Effect};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderPurpose {
    SaveCollection(String),
    OpenCollection,
}

impl AppState {
    pub(super) fn dispatch_folder_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::ChooseFolder(purpose) => effects.push(Effect::PickFolder(purpose)),
            Action::FolderPicked { purpose, path } => match purpose {
                FolderPurpose::SaveCollection(collection_id) => {
                    self.save_collection_to(&collection_id, path, effects)
                }
                FolderPurpose::OpenCollection => self.open_collection_folder(path, effects),
            },
            Action::StopSavingToFolder(collection_id) => {
                self.stop_saving_to_folder(&collection_id, effects)
            }
            _ => {}
        }
    }

    pub fn linked_folder(&self, collection_id: &str) -> Option<&Path> {
        self.store.linked_folder(collection_id)
    }

    fn save_collection_to(
        &mut self,
        collection_id: &str,
        root: PathBuf,
        effects: &mut Vec<Effect>,
    ) {
        let Some(collection) = self.workspace.collection(collection_id) else {
            return;
        };
        match self.store.link_collection(collection, root.clone()) {
            Ok(writes) => {
                effects.extend(writes.into_iter().map(Effect::Persist));
                self.inform(format!(
                    "Saving {} in {}",
                    collection.info.name,
                    root.display()
                ));
            }
            Err(error) => self.report_error(format!("Could not save to the folder: {error}")),
        }
    }

    fn open_collection_folder(&mut self, root: PathBuf, effects: &mut Vec<Effect>) {
        let (collection, owned) = match self.store.open_folder(&root) {
            Ok(opened) => opened,
            Err(error) => {
                self.report_error(format!("Could not open the folder: {error}"));
                return;
            }
        };
        let id = self.workspace.add_collection(collection);
        let remembered = self.store.remember_folder(&id, root, owned);
        self.persist_or_report(remembered, effects);
        self.unsaved.touch_collection(&id, self.now);
    }

    fn stop_saving_to_folder(&mut self, collection_id: &str, effects: &mut Vec<Effect>) {
        if let Some(forgotten) = self.store.unlink_collection(collection_id) {
            self.persist_or_report(forgotten, effects);
            self.unsaved.touch_collection(collection_id, self.now);
        }
    }

    fn persist_or_report(
        &mut self,
        write: Result<StoreWrite, StoreError>,
        effects: &mut Vec<Effect>,
    ) {
        match write {
            Ok(write) => effects.push(Effect::Persist(write)),
            Err(error) => self.report_error(format!("Could not save {error}")),
        }
    }
}
