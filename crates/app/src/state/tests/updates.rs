use requestor_core::update::{Release, Version};
use requestor_core::workspace::Workspace;

use super::state_with;
use crate::state::{Action, Effect, UpdateStage};

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
    state.apply(Action::UpdateAvailable(release()), 0.0);
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
    state.apply(Action::UpdateAvailable(release()), 0.0);
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
    state.apply(Action::UpdateAvailable(release()), 0.0);
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
