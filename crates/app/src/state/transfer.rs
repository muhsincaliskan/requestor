use std::path::{Path, PathBuf};

use requestor_core::collection::Collection;
use requestor_core::environment::Environment;
use requestor_core::import::{import_text, Imported};

use super::overlay::{LibraryTarget, Overlay};
use super::tab::RequestTab;
use super::{Action, AppState, Effect};

const MAX_IMPORT_FILE_BYTES: u64 = 64 * 1024 * 1024;

impl AppState {
    pub(super) fn dispatch_transfer_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::ImportText(text) => self.import(&text),
            Action::ImportFile(path) => self.import_file(&path),
            Action::Export { target, file_path } => self.export(&target, file_path, effects),
            Action::ChooseImportFile => effects.push(Effect::PickImportFile),
            Action::ChooseExportLocation { target, suggested } => {
                effects.push(Effect::PickExportLocation { target, suggested })
            }
            other => self.dispatch_folder_action(other, effects),
        }
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
            Err(error) => self.import_failed(format!("Could not read {}: {error}", path.display())),
        }
    }

    fn import(&mut self, text: &str) {
        let imported = match import_text(text) {
            Ok(imported) => imported,
            Err(error) => {
                self.import_failed(format!("Could not import: {error}"));
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
        if matches!(self.overlay, Some(Overlay::Import(_))) {
            self.overlay = None;
        }
    }

    fn import_failed(&mut self, reason: String) {
        match &mut self.overlay {
            Some(Overlay::Import(form)) => form.error = Some(reason),
            _ => self.report_error(reason),
        }
    }

    fn export(&mut self, target: &LibraryTarget, file_path: PathBuf, effects: &mut Vec<Effect>) {
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
            Some(Err(error)) => self.report_error(format!("Could not export: {error}")),
            None => self.report_error("Nothing to export"),
        }
    }
}
