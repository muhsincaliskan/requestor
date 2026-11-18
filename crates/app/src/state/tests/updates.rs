use requestor_core::update::{Release, Version};
use requestor_core::workspace::Workspace;

use super::state_with;
use crate::state::{Action, Effect};

const PAGE: &str = "https://github.com/muhsincaliskan/requestor/releases/tag/v9.0.0";

fn release() -> Release {
    Release {
        version: Version::parse("9.0.0").unwrap(),
        page_url: PAGE.to_string(),
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
