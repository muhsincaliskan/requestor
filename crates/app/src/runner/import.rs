use std::path::PathBuf;
use std::thread;

use eframe::egui::Context;

use requestor_core::import::{download_address, ImportError, Imported};
use requestor_core::migration::{MigrationReport, PostmanExportFiles};
use requestor_core::settings::RequestSettings;

use super::Runner;
use crate::state::Action;

const IMPORT_WORKER_NAME: &str = "requestor-import";
const IMPORT_SOURCE: &str = "Import";

impl Runner {
    pub fn download_import(
        &self,
        job: u64,
        address: String,
        settings: &RequestSettings,
        wake: &Context,
    ) {
        let settings = settings.clone();
        let report = self.report.clone();
        let worker_wake = wake.clone();
        let spawned = thread::Builder::new()
            .name(IMPORT_WORKER_NAME.to_string())
            .spawn(move || {
                let imported = download_address(&address, &settings);
                let finished = Action::ImportDownloaded {
                    job,
                    imported: Box::new(imported),
                };
                if report.send(finished).is_ok() {
                    worker_wake.request_repaint();
                }
            });
        if let Err(error) = spawned {
            self.report_import(job, Err(ImportError::Download(error.to_string())), wake);
        }
    }

    pub fn import_files(&self, job: u64, paths: Vec<PathBuf>, wake: &Context) {
        let report = self.report.clone();
        let worker_wake = wake.clone();
        let spawned = thread::Builder::new()
            .name(IMPORT_WORKER_NAME.to_string())
            .spawn(move || {
                let imported = PostmanExportFiles::discover(&paths).import();
                let finished = Action::MigrationFinished {
                    job,
                    report: Box::new(imported),
                };
                if report.send(finished).is_ok() {
                    worker_wake.request_repaint();
                }
            });
        if let Err(error) = spawned {
            let mut failed = MigrationReport::default();
            failed.fail(IMPORT_SOURCE, error.to_string());
            let finished = Action::MigrationFinished {
                job,
                report: Box::new(failed),
            };
            if self.report.send(finished).is_ok() {
                wake.request_repaint();
            }
        }
    }

    fn report_import(&self, job: u64, imported: Result<Imported, ImportError>, wake: &Context) {
        let finished = Action::ImportDownloaded {
            job,
            imported: Box::new(imported),
        };
        if self.report.send(finished).is_ok() {
            wake.request_repaint();
        }
    }
}
