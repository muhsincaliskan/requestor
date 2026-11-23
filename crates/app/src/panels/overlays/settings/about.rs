use eframe::egui::{Button, Ui};
use requestor_core::workspace::InterfaceSettings;

use super::row::{row, toggles, Toggle};
use crate::state::{Action, UpdateCheckOrigin, UpdateStage, UpdateStatus};

const NAME: &str = "Requestor";
const TAGLINE: &str = "A lightweight HTTP client.";
const VERSION_LABEL: &str = "Version";
const CREDIT: &str = "Developed by Muhsin Caliskan";
const UPDATES_TITLE: &str = "Updates";
const SECTION_GAP: f32 = 12.0;

const UPDATE_TOGGLES: [Toggle<InterfaceSettings>; 1] = [Toggle {
    label: "Check for updates",
    description: "Look for a newer version on GitHub when the app starts.",
    field: |settings| &mut settings.check_for_updates,
}];

pub struct UpdatePane {
    pub status: UpdateStatus,
    pub can_install: bool,
}

struct UpdateControl {
    label: &'static str,
    action: Option<Action>,
}

pub fn show(
    ui: &mut Ui,
    settings: &mut InterfaceSettings,
    updates: &UpdatePane,
    actions: &mut Vec<Action>,
) -> bool {
    ui.heading(NAME);
    ui.label(TAGLINE);
    ui.label(format!("{VERSION_LABEL} {}", env!("CARGO_PKG_VERSION")));
    ui.weak(CREDIT);
    ui.add_space(SECTION_GAP);
    ui.separator();
    let changed = toggles(ui, &UPDATE_TOGGLES, settings);
    update_row(ui, updates, actions);
    changed
}

fn update_row(ui: &mut Ui, updates: &UpdatePane, actions: &mut Vec<Action>) {
    let control = updates.control();
    row(ui, UPDATES_TITLE, &updates.description(), |ui| {
        let busy = control.action.is_none();
        if ui.add_enabled(!busy, Button::new(control.label)).clicked() {
            actions.extend(control.action);
        }
        if busy {
            ui.spinner();
        }
        false
    });
}

impl UpdatePane {
    fn description(&self) -> String {
        match &self.status {
            UpdateStatus::NotChecked => "Look for a newer version on GitHub.".to_string(),
            UpdateStatus::Checking => "Checking GitHub for a newer version…".to_string(),
            UpdateStatus::UpToDate => {
                format!("Requestor {} is up to date.", env!("CARGO_PKG_VERSION"))
            }
            UpdateStatus::CheckFailed(reason) => format!("Could not check for updates: {reason}"),
            UpdateStatus::Available { version, stage } => match stage {
                UpdateStage::Idle => format!("Requestor {version} is available."),
                UpdateStage::Installing => format!("Installing Requestor {version}…"),
                UpdateStage::Installed => {
                    format!("Requestor {version} is installed. Restart to finish.")
                }
                UpdateStage::Failed(reason) => format!("Could not install {version}: {reason}"),
            },
        }
    }

    fn control(&self) -> UpdateControl {
        let check = UpdateControl {
            label: "Check now",
            action: Some(Action::CheckForUpdates(UpdateCheckOrigin::Manual)),
        };
        match &self.status {
            UpdateStatus::NotChecked | UpdateStatus::UpToDate => check,
            UpdateStatus::CheckFailed(_) => UpdateControl {
                label: "Try again",
                ..check
            },
            UpdateStatus::Checking => UpdateControl {
                label: "Checking…",
                action: None,
            },
            UpdateStatus::Available { stage, .. } => self.install_control(stage),
        }
    }

    fn install_control(&self, stage: &UpdateStage) -> UpdateControl {
        match stage {
            UpdateStage::Installing => UpdateControl {
                label: "Updating…",
                action: None,
            },
            UpdateStage::Installed => UpdateControl {
                label: "Restart",
                action: Some(Action::RestartForUpdate),
            },
            UpdateStage::Idle | UpdateStage::Failed(_) if self.can_install => UpdateControl {
                label: "Update now",
                action: Some(Action::InstallUpdate),
            },
            UpdateStage::Idle | UpdateStage::Failed(_) => UpdateControl {
                label: "Open release page",
                action: Some(Action::OpenReleasePage),
            },
        }
    }
}
