use super::*;
use crate::collection::{Body, Url};

fn entry(name: &str, url: &str) -> HistoryEntry {
    HistoryEntry::new(name, Request::new("GET", Url::Raw(url.to_string())))
}

fn raw_body(text: String) -> Body {
    serde_json::from_value(serde_json::json!({ "mode": "raw", "raw": text })).unwrap()
}

#[test]
fn newest_entry_comes_first() {
    let mut history = History::default();
    history.record(entry("first", "https://example.com/a"));
    history.record(entry("second", "https://example.com/b"));
    let names: Vec<&str> = history.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["second", "first"]);
}

#[test]
fn resending_the_same_request_updates_the_latest_entry() {
    let mut history = History::default();
    history.record(entry("same", "https://example.com").finished(Some(500)));
    history.record(entry("same", "https://example.com").finished(Some(200)));
    assert_eq!(history.len(), 1);
    assert_eq!(history.entries()[0].status, Some(200));
}

#[test]
fn a_changed_request_gets_its_own_entry() {
    let mut history = History::default();
    history.record(entry("same", "https://example.com/a"));
    history.record(entry("same", "https://example.com/b"));
    history.record(entry("same", "https://example.com/a"));
    assert_eq!(history.len(), 3);
}

#[test]
fn history_keeps_only_the_newest_entries() {
    let mut history = History::default();
    for index in 0..HISTORY_LIMIT + 5 {
        history.record(entry(&format!("request {index}"), "https://example.com"));
    }
    assert_eq!(history.len(), HISTORY_LIMIT);
    let newest = format!("request {}", HISTORY_LIMIT + 4);
    assert_eq!(history.entries()[0].name, newest);
}

#[test]
fn a_large_body_is_left_out() {
    let mut request = Request::new("POST", Url::Raw("https://example.com".to_string()));
    request.body = Some(raw_body("x".repeat(MAX_KEPT_BODY_BYTES + 1)));
    let kept = HistoryEntry::new("upload", request);
    assert!(kept.body_left_out);
    assert!(kept.request.body.is_none());
}

#[test]
fn a_small_body_is_kept() {
    let mut request = Request::new("POST", Url::Raw("https://example.com".to_string()));
    request.body = Some(raw_body("{}".to_string()));
    let kept = HistoryEntry::new("create", request);
    assert!(!kept.body_left_out);
    assert!(kept.request.body.is_some());
}

#[test]
fn filter_matches_name_or_url_ignoring_case() {
    let login = entry("Login", "https://api.example.com/session");
    assert!(login.matches_filter("login"));
    assert!(login.matches_filter("SESSION"));
    assert!(login.matches_filter(""));
    assert!(!login.matches_filter("orders"));
}

#[test]
fn history_round_trips_as_a_plain_list() {
    let mut history = History::default();
    history.record(entry("ping", "https://example.com").finished(Some(204)));
    let text = serde_json::to_string(&history).unwrap();
    assert!(text.starts_with('['));
    let restored: History = serde_json::from_str(&text).unwrap();
    assert_eq!(restored, history);
}

#[test]
fn age_reads_in_the_largest_whole_unit() {
    let shown = |seconds: u64| Age::between(1_000, 1_000 + seconds).to_string();
    assert_eq!(shown(5), "just now");
    assert_eq!(shown(125), "2 min ago");
    assert_eq!(shown(3 * 3600 + 10), "3 h ago");
    assert_eq!(shown(2 * 86_400), "2 d ago");
    assert_eq!(Age::between(2_000, 1_000).to_string(), "just now");
}

#[test]
fn the_scheme_is_dropped_from_the_short_url() {
    assert_eq!(
        entry("a", "https://api.example.com/orders").url_without_scheme(),
        "api.example.com/orders"
    );
    assert_eq!(
        entry("b", "{{base}}/orders").url_without_scheme(),
        "{{base}}/orders"
    );
}
