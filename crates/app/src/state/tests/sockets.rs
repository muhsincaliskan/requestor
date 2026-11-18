use std::time::Duration;

use requestor_core::transport::SocketEvent;
use requestor_core::websocket::SocketMessage;

use super::{edit_url, open, response, state_with_request};
use crate::state::{Action, AppState, Effect, Exchange, SocketPhase, TabId};

const SOCKET_URL: &str = "ws://example.test/chat";

fn connect(state: &mut AppState) -> (TabId, u64) {
    let (collection_id, item_id) = {
        let collection = &state.workspace.collections[0];
        (
            collection.id().to_string(),
            collection.item[0].id.clone().unwrap(),
        )
    };
    let tab = open(state, &collection_id, &item_id);
    edit_url(state, tab, SOCKET_URL, 0.0);
    let job = state
        .apply(Action::Send(tab), 0.0)
        .iter()
        .find_map(|effect| match effect {
            Effect::OpenSocket { job, .. } => Some(*job),
            _ => None,
        })
        .expect("connecting opens a socket");
    (tab, job)
}

fn handshake(state: &mut AppState, tab: TabId, job: u64) {
    let mut head = response(b"");
    head.status = 101;
    let event = SocketEvent::Opened(head);
    state.apply(Action::SocketEvent { tab, job, event }, 0.0);
}

fn phase(state: &AppState, tab: TabId) -> SocketPhase {
    let Exchange::Socket(session) = &state.tab(tab).unwrap().exchange else {
        panic!("expected a socket session");
    };
    session.phase().clone()
}

fn sent_texts(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::SocketSend { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_websocket_url_connects_and_the_handshake_goes_into_history() {
    let (mut state, _, _) = state_with_request();
    let (tab, job) = connect(&mut state);
    assert_eq!(phase(&state, tab), SocketPhase::Connecting);
    assert!(state.tab(tab).unwrap().is_connected());
    handshake(&mut state, tab, job);
    assert_eq!(phase(&state, tab), SocketPhase::Open);
    let entry = &state.workspace.history.entries()[0];
    assert_eq!(entry.status, Some(101));
}

#[test]
fn messages_are_sent_from_the_draft_and_logged_as_they_arrive() {
    let (mut state, _, _) = state_with_request();
    let (tab, job) = connect(&mut state);
    handshake(&mut state, tab, job);
    assert!(sent_texts(&state.apply(Action::SendSocketMessage(tab), 0.0)).is_empty());
    state.tab_mut(tab).unwrap().message_draft = "ping".to_string();
    let effects = state.apply(Action::SendSocketMessage(tab), 0.0);
    assert_eq!(sent_texts(&effects), ["ping"]);
    let resent = state.apply(Action::Send(tab), 0.0);
    assert_eq!(sent_texts(&resent), ["ping"]);
    let event = SocketEvent::Message(SocketMessage::sent("ping", Duration::ZERO));
    state.apply(Action::SocketEvent { tab, job, event }, 0.0);
    let Exchange::Socket(session) = &state.tab(tab).unwrap().exchange else {
        panic!("expected a socket session");
    };
    assert_eq!(session.log().total(), 1);
}

#[test]
fn disconnecting_closes_the_socket_and_waits_for_the_close() {
    let (mut state, _, _) = state_with_request();
    let (tab, job) = connect(&mut state);
    handshake(&mut state, tab, job);
    let effects = state.apply(Action::CancelSend(tab), 0.0);
    assert!(effects
        .iter()
        .any(|effect| matches!(effect, Effect::AbortSend(closed) if *closed == job)));
    assert_eq!(phase(&state, tab), SocketPhase::Closing);
    let event = SocketEvent::Closed(Some("1000".to_string()));
    state.apply(Action::SocketEvent { tab, job, event }, 0.0);
    assert_eq!(
        phase(&state, tab),
        SocketPhase::Closed(Some("1000".to_string()))
    );
    assert!(!state.tab(tab).unwrap().is_connected());
}

#[test]
fn disconnecting_while_connecting_ignores_a_late_handshake() {
    let (mut state, _, _) = state_with_request();
    let (tab, job) = connect(&mut state);
    state.apply(Action::CancelSend(tab), 0.0);
    assert_eq!(phase(&state, tab), SocketPhase::Closed(None));
    handshake(&mut state, tab, job);
    assert_eq!(phase(&state, tab), SocketPhase::Closed(None));
}

#[test]
fn a_failed_connection_shows_its_reason() {
    let (mut state, _, _) = state_with_request();
    let (tab, job) = connect(&mut state);
    let result = Err("connection refused".to_string());
    state.apply(Action::SocketEnded { tab, job, result }, 0.0);
    assert_eq!(
        phase(&state, tab),
        SocketPhase::Failed("connection refused".to_string())
    );
    assert_eq!(state.workspace.history.entries()[0].status, None);
}

#[test]
fn events_from_an_older_connection_are_ignored() {
    let (mut state, _, _) = state_with_request();
    let (tab, job) = connect(&mut state);
    handshake(&mut state, tab, job + 100);
    assert_eq!(phase(&state, tab), SocketPhase::Connecting);
}

#[test]
fn closing_the_tab_closes_its_socket() {
    let (mut state, _, _) = state_with_request();
    let (tab, job) = connect(&mut state);
    handshake(&mut state, tab, job);
    let effects = state.apply(Action::ForceCloseTab(tab), 0.0);
    assert!(effects
        .iter()
        .any(|effect| matches!(effect, Effect::AbortSend(closed) if *closed == job)));
}
