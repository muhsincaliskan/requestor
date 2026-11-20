use std::thread;

use eframe::egui::Context;
use requestor_core::migration::MigrationReport;
use requestor_core::postman_cloud::{migrate_workspace, PostmanApiClient, PostmanCloudClient};
use requestor_core::secret::Secret;
use requestor_core::settings::RequestSettings;

use super::Runner;
use crate::state::Action;

const POSTMAN_WORKER_NAME: &str = "requestor-postman";
const WORKSPACE_SOURCE: &str = "Postman workspace";

impl Runner {
    pub fn list_postman_workspaces(
        &self,
        job: u64,
        key: Secret,
        settings: &RequestSettings,
        wake: &Context,
    ) {
        let settings = settings.clone();
        let failed = move |reason| Action::PostmanWorkspacesListed {
            job,
            listed: Err(reason),
        };
        self.spawn_postman_worker(wake, failed, move || {
            let listed = PostmanApiClient::connect(key, &settings)
                .and_then(|client| client.workspaces())
                .map_err(|error| error.to_string());
            Action::PostmanWorkspacesListed { job, listed }
        });
    }

    pub fn migrate_postman_workspace(
        &self,
        job: u64,
        key: Secret,
        workspace_id: String,
        settings: &RequestSettings,
        wake: &Context,
    ) {
        let settings = settings.clone();
        let progress_report = self.report.clone();
        let progress_wake = wake.clone();
        self.spawn_postman_worker(
            wake,
            move |reason| Action::MigrationFinished {
                job,
                report: Box::new(MigrationReport::failed(WORKSPACE_SOURCE, reason)),
            },
            move || {
                let mut progressed = |progress| {
                    if progress_report
                        .send(Action::MigrationProgressed { job, progress })
                        .is_ok()
                    {
                        progress_wake.request_repaint();
                    }
                };
                let migrated = PostmanApiClient::connect(key, &settings)
                    .and_then(|client| migrate_workspace(&client, &workspace_id, &mut progressed))
                    .unwrap_or_else(|error| {
                        MigrationReport::failed(WORKSPACE_SOURCE, error.to_string())
                    });
                Action::MigrationFinished {
                    job,
                    report: Box::new(migrated),
                }
            },
        );
    }

    fn spawn_postman_worker(
        &self,
        wake: &Context,
        failed: impl FnOnce(String) -> Action,
        work: impl FnOnce() -> Action + Send + 'static,
    ) {
        let report = self.report.clone();
        let worker_wake = wake.clone();
        let spawned = thread::Builder::new()
            .name(POSTMAN_WORKER_NAME.to_string())
            .spawn(move || {
                if report.send(work()).is_ok() {
                    worker_wake.request_repaint();
                }
            });
        if let Err(error) = spawned {
            if self.report.send(failed(error.to_string())).is_ok() {
                wake.request_repaint();
            }
        }
    }
}
