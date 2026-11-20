use requestor_core::postman_cloud::MigrationProgress;

use super::migration::ALREADY_RUNNING;
use super::overlay::Overlay;
use super::{Action, AppState, Effect};

impl AppState {
    pub(super) fn dispatch_postman_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::OpenPostmanMigration => self.open_postman_migration(effects),
            Action::PostmanKeyLoaded(key) => self.postman.accept_saved_key(key),
            Action::ListPostmanWorkspaces => self.list_postman_workspaces(effects),
            Action::PostmanWorkspacesListed { job, listed } => {
                if self.postman.finish_listing(job, listed) {
                    effects.push(Effect::StorePostmanKey(self.postman.kept_key()));
                }
            }
            Action::MigratePostmanWorkspace => self.migrate_postman_workspace(effects),
            Action::MigrationProgressed { job, progress } => self.show_progress(job, progress),
            other => self.dispatch_folder_action(other, effects),
        }
    }

    fn open_postman_migration(&mut self, effects: &mut Vec<Effect>) {
        self.overlay = Some(Overlay::PostmanMigration);
        if self.postman.needs_key_lookup() {
            effects.push(Effect::LoadPostmanKey);
        }
    }

    fn list_postman_workspaces(&mut self, effects: &mut Vec<Effect>) {
        let Some(key) = self.postman.key().filter(|_| self.postman.can_list()) else {
            return;
        };
        self.next_job += 1;
        let job = self.next_job;
        self.postman.start_listing(job);
        effects.push(Effect::ListPostmanWorkspaces {
            job,
            key,
            settings: Box::new(self.workspace.settings.request.clone()),
        });
    }

    fn migrate_postman_workspace(&mut self, effects: &mut Vec<Effect>) {
        if self.migration.is_working() {
            self.warn(ALREADY_RUNNING);
            return;
        }
        let Some(key) = self.postman.key() else {
            return;
        };
        let Some(workspace_id) = self
            .postman
            .selected_workspace()
            .map(|workspace| workspace.id.clone())
        else {
            return;
        };
        self.next_job += 1;
        let job = self.next_job;
        self.migration.start(job);
        self.postman.progress = None;
        effects.push(Effect::MigratePostmanWorkspace {
            job,
            key,
            workspace_id,
            settings: Box::new(self.workspace.settings.request.clone()),
        });
    }

    fn show_progress(&mut self, job: u64, progress: MigrationProgress) {
        if self.migration.runs(job) {
            self.postman.progress = Some(progress);
        }
    }
}
