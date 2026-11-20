use std::time::Duration;

use eframe::egui::{Align, Color32, Layout, Ui};

use crate::state::{Action, AppState, NoticeLevel, SaveStatus, UpdateStage};
use crate::widgets::icon_button;

const DISMISS_LABEL: &str = "×";
const DISMISS_HINT: &str = "Dismiss";
const UPDATE_HINT: &str = "Open the download page";
const INSTALL_HINT: &str = "Download, check and install the new version";
const RESTART_LABEL: &str = "Restart to finish updating";
const RELEASE_NOTES_LABEL: &str = "What's new";

pub fn show(ui: &mut Ui, state: &AppState, now: f64, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        match state.notice(now) {
            Some(notice) => {
                ui.colored_label(level_color(ui, notice.level), &notice.text);
                if icon_button(ui, DISMISS_LABEL, DISMISS_HINT).clicked() {
                    actions.push(Action::DismissNotice);
                }
            }
            None => {
                ui.weak(save_status_text(state.save_status()));
            }
        }
        update_link(ui, state, actions);
    });
    if let Some(wait) = state.notice_expires_in(now) {
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(wait.max(0.0)));
    }
}

fn update_link(ui: &mut Ui, state: &AppState, actions: &mut Vec<Action>) {
    let Some(release) = &state.available_update else {
        return;
    };
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if !state.can_install_updates {
            let text = format!("Requestor {} is available", release.version);
            if ui.link(text).on_hover_text(UPDATE_HINT).clicked() {
                actions.push(Action::OpenReleasePage);
            }
            return;
        }
        if ui
            .link(RELEASE_NOTES_LABEL)
            .on_hover_text(UPDATE_HINT)
            .clicked()
        {
            actions.push(Action::OpenReleasePage);
        }
        install_control(ui, state, &release.version.to_string(), actions);
    });
}

fn install_control(ui: &mut Ui, state: &AppState, version: &str, actions: &mut Vec<Action>) {
    match state.update_stage {
        UpdateStage::Installing => {
            ui.weak(format!("Updating to {version}…"));
            ui.spinner();
        }
        UpdateStage::Installed => {
            if ui.small_button(RESTART_LABEL).clicked() {
                actions.push(Action::RestartForUpdate);
            }
        }
        UpdateStage::Idle | UpdateStage::Failed(_) => {
            let label = format!("Update to {version}");
            if ui.small_button(label).on_hover_text(INSTALL_HINT).clicked() {
                actions.push(Action::InstallUpdate);
            }
        }
    }
}

fn level_color(ui: &Ui, level: NoticeLevel) -> Color32 {
    match level {
        NoticeLevel::Info => ui.visuals().text_color(),
        NoticeLevel::Warning => ui.visuals().warn_fg_color,
        NoticeLevel::Error => ui.visuals().error_fg_color,
    }
}

fn save_status_text(status: SaveStatus) -> &'static str {
    match status {
        SaveStatus::Saved => "All changes saved",
        SaveStatus::Saving => "Saving…",
        SaveStatus::Unsaved => "Unsaved changes",
    }
}
