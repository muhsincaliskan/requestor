use super::{Action, AppState, Effect};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateStage {
    #[default]
    Idle,
    Installing,
    Installed,
    Failed(String),
}

impl UpdateStage {
    pub fn can_start(&self) -> bool {
        matches!(self, UpdateStage::Idle | UpdateStage::Failed(_))
    }
}

impl AppState {
    pub(super) fn dispatch_update_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::UpdateAvailable(release) => self.available_update = Some(release),
            Action::OpenReleasePage => effects.extend(self.release_page()),
            Action::InstallUpdate => effects.extend(self.install_update()),
            Action::UpdateInstalled(result) => self.finish_update(result),
            Action::RestartForUpdate => {
                if self.update_stage == UpdateStage::Installed {
                    effects.push(Effect::Relaunch);
                }
            }
            other => self.dispatch_tab_action(other, effects),
        }
    }

    pub(super) fn release_page(&self) -> Option<Effect> {
        let release = self.available_update.as_ref()?;
        Some(Effect::OpenUrl(release.page_url.clone()))
    }

    fn install_update(&mut self) -> Option<Effect> {
        if !self.can_install_updates || !self.update_stage.can_start() {
            return None;
        }
        let release = self.available_update.clone()?;
        self.update_stage = UpdateStage::Installing;
        Some(Effect::InstallUpdate {
            release: Box::new(release),
            settings: Box::new(self.workspace.settings.request.clone()),
        })
    }

    fn finish_update(&mut self, result: Result<(), String>) {
        self.update_stage = match result {
            Ok(()) => UpdateStage::Installed,
            Err(reason) => {
                self.report_error(format!("Could not update Requestor: {reason}"));
                UpdateStage::Failed(reason)
            }
        };
    }
}
