use requestor_core::update::{Release, Version};
use requestor_core::workspace::Workspace;

use super::state_with;
use crate::state::{Action, Effect, NoticeLevel, UpdateCheckOrigin, UpdateStage};

const PAGE: &str = "https://github.com/muhsincaliskan/requestor/releases/tag/v9.0.0";

fn release() -> Release {
    Release {
        version: Version::parse("9.0.0").unwrap(),
        page_url: PAGE.to_string(),
        download_base: String::new(),
    }
}

#[test]
fn a_found_release_opens_its_page() {
    let mut state = state_with(Workspace::default());
    state.apply(Action::UpdateChecked(Ok(Some(release()))), 0.0);
    assert_eq!(state.available_update, Some(release()));
    let effects = state.apply(Action::OpenReleasePage, 0.0);
    assert!(matches!(effects.as_slice(), [Effect::OpenUrl(url)] if url == PAGE));
}

#[test]
fn nothing_opens_without_a_release() {
    let mut state = state_with(Workspace::default());
    assert!(state.apply(Action::OpenReleasePage, 0.0).is_empty());
}

#[test]
fn update_checks_are_on_by_default() {
    let state = state_with(Workspace::default());
    assert!(state.interface().check_for_updates);
}

#[test]
fn installing_runs_once_and_offers_a_restart_when_done() {
    let mut state = state_with(Workspace::default());
    state.can_install_updates = true;
    assert!(state.apply(Action::InstallUpdate, 0.0).is_empty());
    state.apply(Action::UpdateChecked(Ok(Some(release()))), 0.0);
    let effects = state.apply(Action::InstallUpdate, 0.0);
    assert!(
        matches!(effects.as_slice(), [Effect::InstallUpdate { release: found, .. }] if **found == release())
    );
    assert!(state.apply(Action::InstallUpdate, 0.0).is_empty());
    assert!(state.apply(Action::RestartForUpdate, 0.0).is_empty());
    state.apply(Action::UpdateInstalled(Ok(())), 0.0);
    assert_eq!(state.update_stage, UpdateStage::Installed);
    let restart = state.apply(Action::RestartForUpdate, 0.0);
    assert!(matches!(restart.as_slice(), [Effect::Relaunch]));
}

#[test]
fn a_failed_update_can_be_retried_and_installs_need_support() {
    let mut state = state_with(Workspace::default());
    state.apply(Action::UpdateChecked(Ok(Some(release()))), 0.0);
    assert!(state.apply(Action::InstallUpdate, 0.0).is_empty());
    state.can_install_updates = true;
    state.apply(Action::InstallUpdate, 0.0);
    state.apply(Action::UpdateInstalled(Err("offline".to_string())), 0.0);
    assert_eq!(
        state.update_stage,
        UpdateStage::Failed("offline".to_string())
    );
    assert!(state.notice(0.0).is_some());
    assert_eq!(state.apply(Action::InstallUpdate, 0.0).len(), 1);
}

fn notice_level(state: &crate::state::AppState) -> Option<NoticeLevel> {
    state.notice(0.0).map(|notice| notice.level)
}

#[test]
fn the_startup_check_follows_the_setting_and_stays_quiet() {
    let mut state = state_with(Workspace::default());
    let effects = state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Startup), 0.0);
    assert!(matches!(effects.as_slice(), [Effect::CheckForUpdate(_)]));
    state.apply(Action::UpdateChecked(Ok(None)), 0.0);
    assert!(state.notice(0.0).is_none());
    assert!(!state.is_checking_for_updates());

    state.workspace.settings.interface.check_for_updates = false;
    let skipped = state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Startup), 0.0);
    assert!(skipped.is_empty());
}

#[test]
fn a_manual_check_runs_even_when_startup_checks_are_off_and_reports_the_outcome() {
    let mut state = state_with(Workspace::default());
    state.workspace.settings.interface.check_for_updates = false;
    let effects = state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Manual), 0.0);
    assert!(matches!(effects.as_slice(), [Effect::CheckForUpdate(_)]));
    assert!(state.is_checking_for_updates());
    assert!(state
        .apply(Action::CheckForUpdates(UpdateCheckOrigin::Manual), 0.0)
        .is_empty());
    state.apply(Action::UpdateChecked(Ok(None)), 0.0);
    assert_eq!(notice_level(&state), Some(NoticeLevel::Info));
    assert!(state.available_update.is_none());
}

#[test]
fn a_manual_check_during_the_startup_check_reports_its_result() {
    let mut state = state_with(Workspace::default());
    state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Startup), 0.0);
    let joined = state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Manual), 0.0);
    assert!(joined.is_empty());
    state.apply(Action::UpdateChecked(Ok(Some(release()))), 0.0);
    assert_eq!(notice_level(&state), Some(NoticeLevel::Info));
    assert_eq!(state.available_update, Some(release()));
}

#[test]
fn a_failed_manual_check_reports_an_error() {
    let mut state = state_with(Workspace::default());
    state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Manual), 0.0);
    state.apply(Action::UpdateChecked(Err("offline".to_string())), 0.0);
    assert_eq!(notice_level(&state), Some(NoticeLevel::Error));
    assert!(!state.is_checking_for_updates());
}
