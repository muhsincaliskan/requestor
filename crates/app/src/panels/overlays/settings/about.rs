use eframe::egui::{Button, Ui};
use requestor_core::workspace::InterfaceSettings;

use super::row::{row, toggles, Toggle};
use crate::state::{Action, UpdateCheckOrigin};

const NAME: &str = "Requestor";
const TAGLINE: &str = "A lightweight HTTP client.";
const VERSION_LABEL: &str = "Version";
const CREDIT: &str = "Developed by Muhsin Caliskan";
const CHECK_NOW_TITLE: &str = "Check for updates now";
const CHECK_NOW_LABEL: &str = "Check now";
const CHECKING_LABEL: &str = "Checking…";
const SECTION_GAP: f32 = 12.0;

const UPDATE_TOGGLES: [Toggle<InterfaceSettings>; 1] = [Toggle {
    label: "Check for updates",
    description: "Look for a newer version on GitHub when the app starts.",
    field: |settings| &mut settings.check_for_updates,
}];

pub fn show(
    ui: &mut Ui,
    settings: &mut InterfaceSettings,
    checking_for_updates: bool,
    actions: &mut Vec<Action>,
) -> bool {
    ui.heading(NAME);
    ui.label(TAGLINE);
    ui.label(format!("{VERSION_LABEL} {}", env!("CARGO_PKG_VERSION")));
    ui.weak(CREDIT);
    ui.add_space(SECTION_GAP);
    ui.separator();
    let changed = toggles(ui, &UPDATE_TOGGLES, settings);
    check_now(ui, checking_for_updates, actions);
    changed
}

fn check_now(ui: &mut Ui, checking: bool, actions: &mut Vec<Action>) {
    row(ui, CHECK_NOW_TITLE, "", |ui| {
        let label = if checking {
            CHECKING_LABEL
        } else {
            CHECK_NOW_LABEL
        };
        if ui.add_enabled(!checking, Button::new(label)).clicked() {
            actions.push(Action::CheckForUpdates(UpdateCheckOrigin::Manual));
        }
        false
    });
}
