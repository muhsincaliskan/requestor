use requestor_core::update::{Release, Version};
use requestor_core::workspace::Workspace;

use super::state_with;
use crate::state::{Action, Effect, UpdateCheckOrigin, UpdateStage, UpdateStatus};

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

#[test]
fn the_startup_check_follows_the_setting_and_stays_quiet() {
    let mut state = state_with(Workspace::default());
    let effects = state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Startup), 0.0);
    assert!(matches!(effects.as_slice(), [Effect::CheckForUpdate(_)]));
    assert_eq!(state.update_status(), UpdateStatus::Checking);
    state.apply(Action::UpdateChecked(Ok(None)), 0.0);
    assert!(state.notice(0.0).is_none());
    assert_eq!(state.update_status(), UpdateStatus::UpToDate);

    state.workspace.settings.interface.check_for_updates = false;
    let skipped = state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Startup), 0.0);
    assert!(skipped.is_empty());
}

#[test]
fn a_failed_startup_check_is_not_shown() {
    let mut state = state_with(Workspace::default());
    state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Startup), 0.0);
    state.apply(Action::UpdateChecked(Err("offline".to_string())), 0.0);
    assert_eq!(state.update_status(), UpdateStatus::NotChecked);
}

#[test]
fn a_manual_check_runs_even_when_startup_checks_are_off() {
    let mut state = state_with(Workspace::default());
    state.workspace.settings.interface.check_for_updates = false;
    let effects = state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Manual), 0.0);
    assert!(matches!(effects.as_slice(), [Effect::CheckForUpdate(_)]));
    assert!(state
        .apply(Action::CheckForUpdates(UpdateCheckOrigin::Manual), 0.0)
        .is_empty());
    state.apply(Action::UpdateChecked(Ok(None)), 0.0);
    assert_eq!(state.update_status(), UpdateStatus::UpToDate);
}

#[test]
fn a_manual_check_during_the_startup_check_shows_its_failure() {
    let mut state = state_with(Workspace::default());
    state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Startup), 0.0);
    let joined = state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Manual), 0.0);
    assert!(joined.is_empty());
    state.apply(Action::UpdateChecked(Err("offline".to_string())), 0.0);
    assert_eq!(
        state.update_status(),
        UpdateStatus::CheckFailed("offline".to_string())
    );
}

#[test]
fn a_found_release_follows_the_install_stage() {
    let mut state = state_with(Workspace::default());
    state.can_install_updates = true;
    state.apply(Action::CheckForUpdates(UpdateCheckOrigin::Manual), 0.0);
    state.apply(Action::UpdateChecked(Ok(Some(release()))), 0.0);
    let available = |stage| UpdateStatus::Available {
        version: release().version,
        stage,
    };
    assert_eq!(state.update_status(), available(UpdateStage::Idle));
    state.apply(Action::InstallUpdate, 0.0);
    assert_eq!(state.update_status(), available(UpdateStage::Installing));
    state.apply(Action::UpdateInstalled(Ok(())), 0.0);
    assert_eq!(state.update_status(), available(UpdateStage::Installed));
}
