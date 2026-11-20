use std::path::PathBuf;

use requestor_core::migration::{MigrationReport, PostmanExportFiles, WorkspaceArchive};

use super::overlay::Overlay;
use super::tab::RequestTab;
use super::{Action, AppState, Effect};

pub(super) const ALREADY_RUNNING: &str = "An import is already running";
const EXPORTED_TO: &str = "Exported the workspace to";
const EXPORT_FAILED: &str = "Could not export the workspace:";

impl AppState {
    pub(super) fn dispatch_migration_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::FilesDropped(paths) => self.import_dropped(paths, effects),
            Action::ImportFiles(paths) => self.import_files(paths, effects),
            Action::ChooseImportFiles => effects.push(Effect::PickImportFiles),
            Action::ChooseWorkspaceArchiveLocation => {
                effects.push(Effect::PickWorkspaceArchiveLocation)
            }
            Action::ExportWorkspaceArchive(path) => self.export_workspace_archive(path, effects),
            Action::MigrationFinished { job, report } => self.finish_migration(job, *report),
            other => self.dispatch_postman_action(other, effects),
        }
    }

    fn import_dropped(&mut self, mut paths: Vec<PathBuf>, effects: &mut Vec<Effect>) {
        if paths.len() == 1 && !paths[0].is_dir() && !PostmanExportFiles::is_archive(&paths[0]) {
            self.dispatch_transfer_action(Action::ImportFile(paths.remove(0)), effects);
        } else if !paths.is_empty() {
            self.import_files(paths, effects);
        }
    }

    pub(super) fn import_files(&mut self, paths: Vec<PathBuf>, effects: &mut Vec<Effect>) {
        if self.migration.is_working() {
            self.warn(ALREADY_RUNNING);
            return;
        }
        self.next_job += 1;
        let job = self.next_job;
        self.migration.start(job);
        if matches!(
            self.overlay,
            None | Some(Overlay::Import(_)) | Some(Overlay::PostmanMigration)
        ) {
            self.overlay = Some(Overlay::PostmanMigration);
        }
        effects.push(Effect::ImportFiles { job, paths });
    }

    fn export_workspace_archive(&mut self, path: PathBuf, effects: &mut Vec<Effect>) {
        match WorkspaceArchive::of(&self.workspace, self.migration.include_credentials) {
            Ok(contents) => {
                self.inform(format!("{EXPORTED_TO} {}", path.display()));
                effects.push(Effect::WriteArchive { path, contents });
            }
            Err(error) => self.report_error(format!("{EXPORT_FAILED} {error}")),
        }
    }

    fn finish_migration(&mut self, job: u64, report: MigrationReport) {
        self.migration.finish(job, &report);
        self.postman.progress = None;
        let summary = self
            .migration
            .outcome
            .as_ref()
            .map(|outcome| outcome.summary.clone());
        for collection in report.collections {
            let id = self.workspace.add_collection(collection);
            self.unsaved.touch_collection(&id, self.now);
        }
        for environment in report.environments {
            let id = self.workspace.add_environment(environment);
            self.unsaved.touch_environment(&id, self.now);
        }
        for imported in report.requests {
            let id = self.allocate_tab_id();
            self.open_tab(RequestTab::for_request(
                id,
                &imported.name,
                &imported.request,
            ));
        }
        if let Some(summary) = summary {
            self.inform(summary);
        }
    }
}
