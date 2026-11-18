use super::{edit_url, open, response, state_with_request, LATER};
use crate::state::{Action, AppState, Effect, TabId};

const HISTORY_FILE: &str = "history.json";

fn start_send(state: &mut AppState, tab: TabId) -> u64 {
    state
        .apply(Action::Send(tab), 0.0)
        .iter()
        .find_map(|effect| match effect {
            Effect::Send { job, .. } => Some(*job),
            _ => None,
        })
        .unwrap()
}

fn finish(state: &mut AppState, tab: TabId, job: u64, result: Result<&[u8], &str>) {
    let result = result.map(response).map_err(|reason| reason.to_string());
    state.apply(Action::SendFinished { tab, job, result }, 0.0);
}

fn writes_history(effects: &[Effect]) -> bool {
    effects.iter().any(
        |effect| matches!(effect, Effect::Persist(write) if write.path().ends_with(HISTORY_FILE)),
    )
}

#[test]
fn an_answered_send_is_recorded_and_saved() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    let job = start_send(&mut state, tab);
    assert!(state.workspace.history.is_empty());
    finish(&mut state, tab, job, Ok(b"{}"));

    let entries = state.workspace.history.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "list");
    assert_eq!(entries[0].status, Some(200));
    assert!(writes_history(&state.tick(LATER)));
}

#[test]
fn a_failed_send_is_recorded_without_a_status() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    let job = start_send(&mut state, tab);
    finish(&mut state, tab, job, Err("refused"));
    assert_eq!(state.workspace.history.entries()[0].status, None);
}

#[test]
fn a_cancelled_send_is_not_recorded() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    let job = start_send(&mut state, tab);
    state.apply(Action::CancelSend(tab), 0.0);
    finish(&mut state, tab, job, Ok(b"late"));
    assert!(state.workspace.history.is_empty());
}

#[test]
fn the_request_is_recorded_as_it_was_when_sent() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    let job = start_send(&mut state, tab);
    edit_url(&mut state, tab, "https://example.test/changed", 0.0);
    finish(&mut state, tab, job, Ok(b"{}"));
    let recorded = &state.workspace.history.entries()[0];
    assert_eq!(recorded.url_text(), "https://example.test/users");
}

#[test]
fn opening_an_entry_starts_a_new_unsaved_tab_with_that_request() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    let job = start_send(&mut state, tab);
    finish(&mut state, tab, job, Ok(b"{}"));
    let entry = state.workspace.history.entries()[0].clone();

    state.apply(Action::OpenHistoryEntry(Box::new(entry)), 0.0);
    let opened = state.active().unwrap();
    assert_ne!(opened.id, tab);
    assert!(opened.binding.is_none());
    assert!(opened.unsaved);
    assert_eq!(opened.draft.name, "list");
    assert_eq!(opened.draft.url, "https://example.test/users");
}

#[test]
fn clearing_empties_the_history_and_saves_it() {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    let job = start_send(&mut state, tab);
    finish(&mut state, tab, job, Ok(b"{}"));
    state.tick(LATER);

    state.apply(Action::ClearHistory, LATER);
    assert!(state.workspace.history.is_empty());
    assert!(writes_history(&state.tick(LATER * 2.0)));
}
