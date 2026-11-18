use std::path::{Path, PathBuf};

use requestor_core::collection::Collection;
use requestor_core::environment::Environment;
use requestor_core::import::wsdl::is_wsdl_address;
use requestor_core::import::{import_file_text, import_text, ImportError, Imported};

use super::export_format::ExportFormat;
use super::overlay::{ImportForm, LibraryTarget, Overlay};
use super::tab::RequestTab;
use super::{Action, AppState, Effect};

const MAX_IMPORT_FILE_BYTES: u64 = 64 * 1024 * 1024;

impl AppState {
    pub(super) fn dispatch_transfer_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::ImportText(text) if is_wsdl_address(&text) => {
                self.download_import(text, effects)
            }
            Action::ImportText(text) => self.import(import_text(&text)),
            Action::ImportDownloaded { job, imported } => self.finish_download(job, *imported),
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
            Ok(text) => self.import(import_file_text(&text, path)),
            Err(error) => self.import_failed(format!("Could not read {}: {error}", path.display())),
        }
    }

    fn import(&mut self, imported: Result<Imported, ImportError>) {
        let imported = match imported {
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

    fn download_import(&mut self, address: String, effects: &mut Vec<Effect>) {
        self.next_job += 1;
        let job = self.next_job;
        if !matches!(self.overlay, Some(Overlay::Import(_))) {
            self.overlay = Some(Overlay::Import(ImportForm::with_text(address.clone())));
        }
        if let Some(Overlay::Import(form)) = &mut self.overlay {
            form.start_download(job);
        }
        effects.push(Effect::DownloadImport {
            job,
            address: address.trim().to_string(),
            settings: Box::new(self.workspace.settings.request.clone()),
        });
    }

    fn finish_download(&mut self, job: u64, imported: Result<Imported, ImportError>) {
        let current = match &mut self.overlay {
            Some(Overlay::Import(form)) => form.finish_download(job),
            _ => false,
        };
        if current {
            self.import(imported);
        }
    }

    fn import_failed(&mut self, reason: String) {
        match &mut self.overlay {
            Some(Overlay::Import(form)) => form.error = Some(reason),
            _ => self.report_error(reason),
        }
    }

    pub fn exposed_credential_count(&self, target: &LibraryTarget) -> usize {
        match target {
            LibraryTarget::Collection { collection_id } => self
                .workspace
                .collection(collection_id)
                .map_or(0, Collection::literal_secret_count),
            LibraryTarget::Item {
                collection_id,
                item_id,
            } => self
                .workspace
                .collection(collection_id)
                .and_then(|collection| collection.find_item(item_id))
                .map_or(0, |item| {
                    Collection::containing(item.clone()).literal_secret_count()
                }),
            LibraryTarget::Environment { environment_id } => self
                .workspace
                .environment(environment_id)
                .map_or(0, Environment::unmarked_secret_count),
        }
    }

    fn export(&mut self, target: &LibraryTarget, file_path: PathBuf, effects: &mut Vec<Effect>) {
        let keep_credentials = matches!(
            &self.overlay,
            Some(Overlay::Export(form)) if form.keep_credentials
        );
        let format = ExportFormat::of(&file_path);
        let settings = &self.workspace.settings.request;
        let shareable = |collection: Collection| {
            if keep_credentials {
                collection
            } else {
                collection.without_literal_secrets()
            }
        };
        let document = match target {
            LibraryTarget::Collection { collection_id } => self
                .workspace
                .collection(collection_id)
                .map(|collection| format.render(&shareable(collection.clone()), settings)),
            LibraryTarget::Item {
                collection_id,
                item_id,
            } => self
                .workspace
                .collection(collection_id)
                .and_then(|collection| collection.find_item(item_id))
                .map(|item| {
                    format.render(&shareable(Collection::containing(item.clone())), settings)
                }),
            LibraryTarget::Environment { environment_id } => self
                .workspace
                .environment(environment_id)
                .map(|environment| {
                    if keep_credentials {
                        environment.to_export_json()
                    } else {
                        environment.without_unmarked_secrets().to_export_json()
                    }
                }),
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
