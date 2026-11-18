use super::{open, response, state_with_request};
use crate::state::{Action, AppState, Effect, Exchange, TabId};

fn send_and_receive(state: &mut AppState, tab: TabId, body: &[u8]) {
    let effects = state.apply(Action::Send(tab), 0.0);
    let [Effect::Send { job, .. }] = effects.as_slice() else {
        panic!("expected one send effect");
    };
    let job = *job;
    state.apply(
        Action::SendFinished {
            tab,
            job,
            result: Ok(response(body)),
        },
        0.0,
    );
}

fn added_and_removed(state: &mut AppState, tab: TabId) -> Option<(usize, usize)> {
    let Exchange::Completed(view) = &mut state.tab_mut(tab).unwrap().exchange else {
        return None;
    };
    let diff = view.changes()?;
    Some((diff.added, diff.removed))
}

#[test]
fn a_second_response_is_compared_with_the_first_one_in_the_same_tab() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    send_and_receive(&mut state, tab, b"one\ntwo");
    let Exchange::Completed(first) = &state.tab(tab).unwrap().exchange else {
        panic!("expected a response");
    };
    assert!(!first.has_previous());

    send_and_receive(&mut state, tab, b"one\n2");
    assert_eq!(added_and_removed(&mut state, tab), Some((1, 1)));
}

#[test]
fn a_failed_send_leaves_nothing_to_compare_with() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    send_and_receive(&mut state, tab, b"one");
    let effects = state.apply(Action::Send(tab), 0.0);
    let [Effect::Send { job, .. }] = effects.as_slice() else {
        panic!("expected one send effect");
    };
    let job = *job;
    state.apply(
        Action::SendFinished {
            tab,
            job,
            result: Err("refused".to_string()),
        },
        0.0,
    );
    send_and_receive(&mut state, tab, b"two");
    assert_eq!(added_and_removed(&mut state, tab), None);
}
