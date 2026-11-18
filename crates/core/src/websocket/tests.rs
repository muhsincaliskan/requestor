use std::time::Duration;

use super::*;

fn received(text: &str) -> SocketMessage {
    SocketMessage::received(SocketPayload::Text(text.to_string()), Duration::ZERO)
}

#[test]
fn websocket_urls_are_recognised_by_scheme() {
    assert!(is_websocket_url("ws://localhost:8080/chat"));
    assert!(is_websocket_url("  WSS://example.com"));
    assert!(!is_websocket_url("https://example.com"));
    assert!(!is_websocket_url("ws:/broken"));
    assert!(!is_websocket_url("{{base}}/ws"));
}

#[test]
fn the_log_keeps_the_newest_messages_and_counts_all() {
    let mut log = SocketLog::default();
    for index in 0..MAX_KEPT_MESSAGES + 3 {
        log.push(received(&index.to_string()));
    }
    assert_eq!(log.total(), MAX_KEPT_MESSAGES + 3);
    assert_eq!(log.kept(), MAX_KEPT_MESSAGES);
    assert_eq!(log.get(0).and_then(SocketMessage::text), Some("3"));
    assert_eq!(log.number_of(0), 4);
}

#[test]
fn the_log_drops_old_messages_to_stay_within_its_byte_budget() {
    let mut log = SocketLog::default();
    let large = "x".repeat(MAX_KEPT_MESSAGE_BYTES / 2 + 1);
    log.push(received(&large));
    log.push(received(&large));
    assert_eq!(log.kept(), 1);
    assert_eq!(log.dropped(), 1);
}

#[test]
fn a_single_oversized_message_is_still_kept() {
    let mut log = SocketLog::default();
    log.push(received(&"x".repeat(MAX_KEPT_MESSAGE_BYTES + 1)));
    assert_eq!(log.kept(), 1);
}

#[test]
fn sent_and_binary_messages_report_their_size() {
    assert_eq!(SocketMessage::sent("hello", Duration::ZERO).size(), 5);
    let binary = SocketMessage::received(SocketPayload::Binary(vec![0; 3]), Duration::ZERO);
    assert_eq!(binary.size(), 3);
    assert_eq!(binary.text(), None);
}
