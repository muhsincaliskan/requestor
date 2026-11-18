use std::path::Path;

use eframe::egui::{Response, Ui};
use requestor_core::collection::{Collection, Item};
use requestor_core::environment::Environment;

use crate::state::{
    Action, DeleteConfirmation, ExportForm, FolderPurpose, LibraryTarget, Overlay, RenameForm,
};

const OPEN_LABEL: &str = "Open";
const ADD_REQUEST_LABEL: &str = "Add request";
const ADD_FOLDER_LABEL: &str = "Add folder";
const EDIT_LABEL: &str = "Edit";
const RUN_LABEL: &str = "Run collection…";
const RENAME_LABEL: &str = "Rename";
const DUPLICATE_LABEL: &str = "Duplicate";
const EXPORT_LABEL: &str = "Export";
const DELETE_LABEL: &str = "Delete";
const SAVE_TO_FOLDER_LABEL: &str = "Save to folder…";
const STOP_FOLDER_LABEL: &str = "Stop saving to folder";
const CLEAR_HISTORY_LABEL: &str = "Clear history";

pub(super) fn collection(
    response: &Response,
    collection: &Collection,
    folder: Option<&Path>,
    actions: &mut Vec<Action>,
) {
    response.context_menu(|ui| {
        let collection_id = collection.id();
        entry(ui, ADD_REQUEST_LABEL, actions, || Action::AddRequest {
            collection_id: collection_id.to_string(),
            parent_id: None,
        });
        entry(ui, ADD_FOLDER_LABEL, actions, || Action::AddFolder {
            collection_id: collection_id.to_string(),
            parent_id: None,
        });
        entry(ui, EDIT_LABEL, actions, || {
            Action::Show(Overlay::CollectionEditor {
                collection_id: collection_id.to_string(),
            })
        });
        entry(ui, RUN_LABEL, actions, || {
            Action::OpenRunner(collection_id.to_string())
        });
        folder_entry(ui, collection_id, folder, actions);
        let target = LibraryTarget::Collection {
            collection_id: collection_id.to_string(),
        };
        library_entries(ui, &target, &collection.info.name, actions);
    });
}

fn folder_entry(
    ui: &mut Ui,
    collection_id: &str,
    folder: Option<&Path>,
    actions: &mut Vec<Action>,
) {
    match folder {
        Some(_) => entry(ui, STOP_FOLDER_LABEL, actions, || {
            Action::StopSavingToFolder(collection_id.to_string())
        }),
        None => entry(ui, SAVE_TO_FOLDER_LABEL, actions, || {
            Action::ChooseFolder(FolderPurpose::SaveCollection(collection_id.to_string()))
        }),
    }
}

pub(super) fn item(
    response: &Response,
    collection_id: &str,
    item: &Item,
    actions: &mut Vec<Action>,
) {
    let Some(item_id) = item.id.as_deref() else {
        return;
    };
    response.context_menu(|ui| {
        item_entries(ui, collection_id, item, actions);
        entry(ui, DUPLICATE_LABEL, actions, || Action::DuplicateItem {
            collection_id: collection_id.to_string(),
            item_id: item_id.to_string(),
        });
        let target = LibraryTarget::Item {
            collection_id: collection_id.to_string(),
            item_id: item_id.to_string(),
        };
        library_entries(ui, &target, item.display_name(), actions);
    });
}

pub(super) fn environment(
    response: &Response,
    environment: &Environment,
    actions: &mut Vec<Action>,
) {
    let Some(environment_id) = environment.id.as_deref() else {
        return;
    };
    response.context_menu(|ui| {
        entry(ui, EDIT_LABEL, actions, || {
            Action::Show(Overlay::EnvironmentEditor {
                environment_id: environment_id.to_string(),
            })
        });
        let target = LibraryTarget::Environment {
            environment_id: environment_id.to_string(),
        };
        library_entries(ui, &target, &environment.name, actions);
    });
}

fn item_entries(ui: &mut Ui, collection_id: &str, item: &Item, actions: &mut Vec<Action>) {
    let Some(item_id) = item.id.as_deref() else {
        return;
    };
    if item.is_folder() {
        let parent_id = Some(item_id.to_string());
        entry(ui, ADD_REQUEST_LABEL, actions, || Action::AddRequest {
            collection_id: collection_id.to_string(),
            parent_id: parent_id.clone(),
        });
        entry(ui, ADD_FOLDER_LABEL, actions, || Action::AddFolder {
            collection_id: collection_id.to_string(),
            parent_id,
        });
        entry(ui, EDIT_LABEL, actions, || {
            Action::Show(Overlay::FolderEditor {
                collection_id: collection_id.to_string(),
                folder_id: item_id.to_string(),
            })
        });
    } else {
        entry(ui, OPEN_LABEL, actions, || Action::OpenItem {
            collection_id: collection_id.to_string(),
            item_id: item_id.to_string(),
        });
    }
}

pub(super) fn history(response: &Response, actions: &mut Vec<Action>) {
    response.context_menu(|ui| entry(ui, CLEAR_HISTORY_LABEL, actions, || Action::ClearHistory));
}

fn library_entries(ui: &mut Ui, target: &LibraryTarget, name: &str, actions: &mut Vec<Action>) {
    entry(ui, RENAME_LABEL, actions, || {
        Action::Show(Overlay::Rename(RenameForm {
            target: target.clone(),
            name: name.to_string(),
        }))
    });
    entry(ui, EXPORT_LABEL, actions, || {
        Action::Show(Overlay::Export(ExportForm::suggested(target.clone(), name)))
    });
    entry(ui, DELETE_LABEL, actions, || {
        Action::Show(Overlay::ConfirmDelete(DeleteConfirmation {
            target: target.clone(),
            name: name.to_string(),
        }))
    });
}

fn entry(ui: &mut Ui, label: &str, actions: &mut Vec<Action>, action: impl FnOnce() -> Action) {
    if ui.button(label).clicked() {
        actions.push(action());
        ui.close();
    }
}
