use std::time::Duration;

use eframe::egui::{Align, Color32, Layout, Ui};

use crate::state::{Action, AppState, NoticeLevel, SaveStatus};
use crate::widgets::icon_button;

const DISMISS_LABEL: &str = "×";
const DISMISS_HINT: &str = "Dismiss";
const UPDATE_HINT: &str = "Open the download page";

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
        let text = format!("Requestor {} is available", release.version);
        if ui.link(text).on_hover_text(UPDATE_HINT).clicked() {
            actions.push(Action::OpenReleasePage);
        }
    });
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
