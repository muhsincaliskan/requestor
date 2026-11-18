use requestor_core::environment::Environment;

use super::{open, state_with_request};
use crate::state::{Action, AppState, Effect, Overlay};

fn with_environment(state: &mut AppState, name: &str) {
    let id = state.workspace.add_environment(Environment::new(name));
    state.apply(Action::SelectEnvironment(Some(id)), 0.0);
}

#[test]
fn sending_to_a_protected_environment_asks_first() {
    let (mut state, collection_id, item_id) = state_with_request();
    with_environment(&mut state, "Production");
    let tab = open(&mut state, &collection_id, &item_id);
    let effects = state.apply(Action::Send(tab), 1.0);
    assert!(effects.is_empty());
    assert_eq!(state.overlay, Some(Overlay::ConfirmProtectedSend(tab)));

    state.overlay = None;
    let effects = state.apply(Action::SendConfirmed(tab), 2.0);
    assert!(matches!(effects.as_slice(), [Effect::Send { .. }]));
}

#[test]
fn other_environments_send_straight_away() {
    let (mut state, collection_id, item_id) = state_with_request();
    with_environment(&mut state, "Staging");
    let tab = open(&mut state, &collection_id, &item_id);
    let effects = state.apply(Action::Send(tab), 1.0);
    assert!(matches!(effects.as_slice(), [Effect::Send { .. }]));
    assert_eq!(state.overlay, None);
}
