use requestor_core::update::{Release, Version};
use requestor_core::workspace::InterfaceSettings;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateCheckOrigin {
    Startup,
    Manual,
}

impl UpdateCheckOrigin {
    fn is_wanted(self, interface: &InterfaceSettings) -> bool {
        self == UpdateCheckOrigin::Manual || interface.check_for_updates
    }

    fn takes_over(self) -> bool {
        self == UpdateCheckOrigin::Manual
    }
}

pub type UpdateCheckResult = Result<Option<Release>, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    NotChecked,
    Checking,
    UpToDate,
    CheckFailed(String),
    Available {
        version: Version,
        stage: UpdateStage,
    },
}

impl AppState {
    pub(super) fn dispatch_update_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::CheckForUpdates(origin) => effects.extend(self.check_for_updates(origin)),
            Action::UpdateChecked(result) => self.finish_update_check(result),
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

    pub fn update_status(&self) -> UpdateStatus {
        if let Some(release) = &self.available_update {
            return UpdateStatus::Available {
                version: release.version,
                stage: self.update_stage.clone(),
            };
        }
        if self.update_check.is_some() {
            return UpdateStatus::Checking;
        }
        match &self.last_update_check {
            None => UpdateStatus::NotChecked,
            Some(Ok(())) => UpdateStatus::UpToDate,
            Some(Err(reason)) => UpdateStatus::CheckFailed(reason.clone()),
        }
    }

    fn check_for_updates(&mut self, origin: UpdateCheckOrigin) -> Option<Effect> {
        if !origin.is_wanted(self.interface()) {
            return None;
        }
        if self.update_check.is_some() {
            if origin.takes_over() {
                self.update_check = Some(origin);
            }
            return None;
        }
        self.update_check = Some(origin);
        Some(Effect::CheckForUpdate(Box::new(
            self.workspace.settings.request.clone(),
        )))
    }

    fn finish_update_check(&mut self, result: UpdateCheckResult) {
        let origin = self.update_check.take();
        match result {
            Ok(Some(release)) => self.available_update = Some(release),
            Ok(None) => self.last_update_check = Some(Ok(())),
            Err(reason) if origin.is_some_and(UpdateCheckOrigin::takes_over) => {
                self.last_update_check = Some(Err(reason))
            }
            Err(_) => {}
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
