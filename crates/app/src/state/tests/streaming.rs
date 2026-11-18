use requestor_core::header::Header;
use requestor_core::response::HttpResponse;

use super::{open, response, state_with_request};
use crate::state::{Action, AppState, Effect, Exchange, ResponseSection, StreamUpdate, TabId};

fn event_stream_head() -> HttpResponse {
    let mut head = response(b"");
    head.headers
        .push(Header::new("Content-Type", "text/event-stream"));
    head
}

fn start_stream(state: &mut AppState) -> (TabId, u64) {
    let (collection_id, item_id) = {
        let collection = &state.workspace.collections[0];
        (
            collection.id().to_string(),
            collection.item[0].id.clone().unwrap(),
        )
    };
    let tab = open(state, &collection_id, &item_id);
    let job = state
        .apply(Action::Send(tab), 0.0)
        .iter()
        .find_map(|effect| match effect {
            Effect::Send { job, .. } => Some(*job),
            _ => None,
        })
        .unwrap();
    let update = StreamUpdate::Opened(Box::new(event_stream_head()));
    state.apply(Action::StreamProgress { tab, job, update }, 0.0);
    (tab, job)
}

fn receive(state: &mut AppState, tab: TabId, job: u64, bytes: &[u8]) {
    let update = StreamUpdate::Received(bytes.to_vec());
    state.apply(Action::StreamProgress { tab, job, update }, 0.0);
}

fn completed_event_count(state: &AppState, tab: TabId) -> Option<usize> {
    let Exchange::Completed(view) = &state.tab(tab)?.exchange else {
        return None;
    };
    view.events().map(|log| log.total())
}

#[test]
fn events_show_while_the_stream_is_open() {
    let (mut state, _, _) = state_with_request();
    let (tab, job) = start_stream(&mut state);
    receive(&mut state, tab, job, b"data: one\n\ndata: tw");
    let opened = state.tab(tab).unwrap();
    assert_eq!(opened.response_section, ResponseSection::Events);
    let Exchange::Sending {
        live: Some(live), ..
    } = &opened.exchange
    else {
        panic!("expected a live stream");
    };
    assert_eq!(live.log().total(), 1);
}

#[test]
fn stopping_keeps_what_was_received_and_ends_the_request() {
    let (mut state, _, _) = state_with_request();
    let (tab, job) = start_stream(&mut state);
    receive(&mut state, tab, job, b"data: one\n\ndata: two\n\n");
    let effects = state.apply(Action::CancelSend(tab), 0.0);
    assert!(effects
        .iter()
        .any(|effect| matches!(effect, Effect::AbortSend(stopped) if *stopped == job)));
    assert_eq!(completed_event_count(&state, tab), Some(2));
    assert_eq!(state.workspace.history.len(), 1);
}

#[test]
fn a_dropped_connection_keeps_the_events_and_warns() {
    let (mut state, _, _) = state_with_request();
    let (tab, job) = start_stream(&mut state);
    receive(&mut state, tab, job, b"data: one\n\n");
    let result = Err("connection reset".to_string());
    state.apply(Action::SendFinished { tab, job, result }, 0.0);
    assert_eq!(completed_event_count(&state, tab), Some(1));
    assert!(state.notice(0.0).is_some());
}

#[test]
fn progress_from_an_older_send_is_ignored() {
    let (mut state, _, _) = state_with_request();
    let (tab, job) = start_stream(&mut state);
    receive(&mut state, tab, job + 100, b"data: stray\n\n");
    let Exchange::Sending {
        live: Some(live), ..
    } = &state.tab(tab).unwrap().exchange
    else {
        panic!("expected a live stream");
    };
    assert_eq!(live.log().total(), 0);
}

#[test]
fn a_finished_stream_lists_its_events() {
    let (mut state, _, _) = state_with_request();
    let (tab, job) = start_stream(&mut state);
    let mut finished = event_stream_head();
    finished.body = b"data: a\n\ndata: b\n\ndata: c\n\n".to_vec();
    state.apply(
        Action::SendFinished {
            tab,
            job,
            result: Ok(finished),
        },
        0.0,
    );
    assert_eq!(completed_event_count(&state, tab), Some(3));
}
