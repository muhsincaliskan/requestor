use super::*;

fn event(name: Option<&str>, data: &str, id: Option<&str>) -> ServerEvent {
    ServerEvent {
        name: name.map(str::to_string),
        data: data.to_string(),
        id: id.map(str::to_string),
    }
}

fn parse_all(body: &[u8]) -> Vec<ServerEvent> {
    EventStreamParser::default().push(body)
}

#[test]
fn events_are_split_on_blank_lines() {
    let body = b"data: first\n\nevent: update\ndata: second\nid: 7\n\n";
    assert_eq!(
        parse_all(body),
        [
            event(None, "first", None),
            event(Some("update"), "second", Some("7"))
        ]
    );
}

#[test]
fn data_lines_are_joined_and_comments_ignored() {
    let body = b": keep-alive\ndata: one\ndata:two\nretry: 10\n\n";
    assert_eq!(parse_all(body), [event(None, "one\ntwo", None)]);
}

#[test]
fn every_line_ending_is_understood() {
    let body = b"data: a\r\n\r\ndata: b\r\rdata: c\n\n";
    let data: Vec<String> = parse_all(body)
        .into_iter()
        .map(|event| event.data)
        .collect();
    assert_eq!(data, ["a", "b", "c"]);
}

#[test]
fn chunks_split_anywhere_give_the_same_events() {
    let body = b"event: tick\r\ndata: {\"n\":1}\r\n\r\nid: 2\ndata: done\n\n";
    let whole = parse_all(body);
    for split in 0..body.len() {
        let mut parser = EventStreamParser::default();
        let mut events = parser.push(&body[..split]);
        events.extend(parser.push(&body[split..]));
        assert_eq!(events, whole, "split at {split}");
    }
}

#[test]
fn the_last_id_carries_over_and_an_unfinished_event_is_not_dispatched() {
    let body = b"id: 1\ndata: a\n\ndata: b\n\ndata: c";
    let events = parse_all(body);
    assert_eq!(
        events,
        [event(None, "a", Some("1")), event(None, "b", Some("1"))]
    );
}

#[test]
fn a_leading_byte_order_mark_is_skipped() {
    let events = parse_all("\u{FEFF}data: a\n\n".as_bytes());
    assert_eq!(events, [event(None, "a", None)]);
}

#[test]
fn an_id_with_a_null_character_is_ignored() {
    let events = parse_all(b"id: 1\n\ndata: a\nid: 2\0\n\n");
    assert_eq!(events, [event(None, "a", Some("1"))]);
}

#[test]
fn events_without_data_are_skipped_and_names_default_to_message() {
    let events = parse_all(b"event: ping\n\ndata: x\n\n");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind(), "message");
}

#[test]
fn the_content_type_is_recognised() {
    assert!(is_event_stream("text/event-stream"));
    assert!(is_event_stream("Text/Event-Stream; charset=utf-8"));
    assert!(!is_event_stream("text/plain"));
}

#[test]
fn the_log_keeps_only_the_newest_events_but_counts_all() {
    let body: Vec<u8> = (0..MAX_KEPT_EVENTS + 5)
        .flat_map(|index| format!("data: {index}\n\n").into_bytes())
        .collect();
    let log = EventLog::from_body(&body);
    assert_eq!(log.total(), MAX_KEPT_EVENTS + 5);
    assert_eq!(log.kept(), MAX_KEPT_EVENTS);
    assert_eq!(log.dropped(), 5);
    assert_eq!(log.get(0).unwrap().data, "5");
    assert_eq!(log.number_of(0), 6);
}

#[test]
fn the_log_grows_as_chunks_arrive() {
    let mut log = EventLog::default();
    log.receive(b"data: a\n");
    assert_eq!(log.total(), 0);
    log.receive(b"\ndata: b\n\n");
    assert_eq!(log.total(), 2);
}
