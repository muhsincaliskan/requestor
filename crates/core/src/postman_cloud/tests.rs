use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::header::Header;
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;
use crate::transport::{HttpTransport, TransportError};

const BASE: &str = "https://postman.test";
const KEY: &str = "PMAK-test";

type Answer = (u16, Vec<Header>, String);

#[derive(Clone, Default)]
struct Script {
    answers: Arc<Mutex<VecDeque<Answer>>>,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Script {
    fn answer(self, status: u16, body: &str) -> Self {
        self.answer_with(status, Vec::new(), body)
    }

    fn answer_with(self, status: u16, headers: Vec<Header>, body: &str) -> Self {
        self.answers
            .lock()
            .unwrap()
            .push_back((status, headers, body.to_string()));
        self
    }

    fn seen(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }
}

impl HttpTransport for Script {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        assert_eq!(request.header("X-Api-Key"), Some(KEY));
        self.seen.lock().unwrap().push(request.url.clone());
        let (status, headers, body) = self
            .answers
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| TransportError::ConnectionFailed("no answer".to_string()))?;
        Ok(HttpResponse {
            status,
            reason: String::new(),
            version: String::new(),
            headers,
            body: body.into_bytes(),
            truncated: false,
            elapsed: Duration::ZERO,
            timing: None,
        })
    }
}

fn client(script: &Script) -> (PostmanApiClient<Script>, Arc<Mutex<Vec<Duration>>>) {
    let pauses = Arc::new(Mutex::new(Vec::new()));
    let recorded = pauses.clone();
    let client = PostmanApiClient::new(script.clone(), BASE, crate::secret::Secret::new(KEY))
        .pausing_with(move |wait| recorded.lock().unwrap().push(wait));
    (client, pauses)
}

const WORKSPACES: &str = r#"{"workspaces":[{"id":"w-1","name":"My Workspace","type":"personal","visibility":"personal"}]}"#;
const WORKSPACE: &str = r#"{"workspace":{"id":"w-1","name":"My Workspace","collections":[{"id":"c","name":"Orders","uid":"1-c"}],"environments":[{"id":"e","name":"Staging","uid":"1-e"}]}}"#;
const COLLECTION: &str = r#"{"collection":{"info":{"_postman_id":"c","name":"Orders","uid":"1-c","createdAt":"2020","custom":"kept","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},"item":[{"name":"Admin","item":[{"name":"List","request":{"method":"GET","url":"https://h/orders"}}]}]}}"#;
const ENVIRONMENT: &str = r#"{"environment":{"id":"e","name":"Staging","owner":"42","createdAt":"2020","updatedAt":"2021","isPublic":false,"custom":"kept","values":[{"key":"host","value":"h","enabled":true,"type":"default"}]}}"#;

#[test]
fn workspaces_are_listed_with_the_api_key() {
    let script = Script::default().answer(200, WORKSPACES);
    let (client, _) = client(&script);

    let workspaces = client.workspaces().unwrap();

    assert_eq!(workspaces[0].name, "My Workspace");
    assert_eq!(workspaces[0].kind, "personal");
    assert_eq!(script.seen(), vec![format!("{BASE}/workspaces")]);
}

#[test]
fn a_workspace_moves_with_folders_and_without_account_metadata() {
    let script = Script::default()
        .answer(200, WORKSPACE)
        .answer(200, COLLECTION)
        .answer(200, ENVIRONMENT);
    let (client, _) = client(&script);
    let mut steps = Vec::new();

    let report = migrate_workspace(&client, "w-1", &mut |step| steps.push(step)).unwrap();

    assert_eq!(report.collections[0].item[0].name.as_deref(), Some("Admin"));
    let environment = &report.environments[0];
    assert_eq!(environment.name, "Staging");
    assert!(!environment.extra.contains_key("owner"));
    assert!(environment.extra.contains_key("custom"));
    let info = &report.collections[0].info.extra;
    assert!(!info.contains_key("uid") && !info.contains_key("createdAt"));
    assert!(info.contains_key("custom"));
    assert_eq!(steps.len(), 2);
    assert_eq!((steps[1].done, steps[1].total), (1, 2));
    assert_eq!(
        script.seen()[1..],
        [
            format!("{BASE}/collections/1-c"),
            format!("{BASE}/environments/1-e")
        ]
    );
}

#[test]
fn one_failing_item_is_reported_and_the_rest_still_moves() {
    let script = Script::default()
        .answer(200, WORKSPACE)
        .answer(500, "{}")
        .answer(200, ENVIRONMENT);
    let (client, _) = client(&script);

    let report = migrate_workspace(&client, "w-1", &mut |_| {}).unwrap();

    assert_eq!(report.environments.len(), 1);
    assert_eq!(report.failures[0].source, "Orders");
    assert!(report.failures[0].reason.contains("500"));
}

#[test]
fn a_rate_limit_waits_as_told_and_then_retries() {
    let wait = vec![Header::new("Retry-After", "2")];
    let script = Script::default()
        .answer_with(429, wait, "{}")
        .answer(200, WORKSPACES);
    let (client, pauses) = client(&script);

    assert_eq!(client.workspaces().unwrap().len(), 1);
    assert_eq!(*pauses.lock().unwrap(), vec![Duration::from_secs(2)]);
}

#[test]
fn a_lasting_rate_limit_stops_the_move_and_names_what_was_skipped() {
    let script = Script::default()
        .answer(200, WORKSPACE)
        .answer(429, "{}")
        .answer(429, "{}")
        .answer(429, "{}")
        .answer(429, "{}");
    let (client, pauses) = client(&script);

    let report = migrate_workspace(&client, "w-1", &mut |_| {}).unwrap();

    assert_eq!(pauses.lock().unwrap().len(), 3);
    assert_eq!(report.imported_count(), 0);
    assert_eq!(report.failures.len(), 2);
    assert_eq!(report.failures[1].source, "Staging");
}

#[test]
fn a_wait_longer_than_a_minute_is_not_sat_out() {
    let wait = vec![Header::new("Retry-After", "86400")];
    let script = Script::default().answer_with(429, wait, "{}");
    let (client, pauses) = client(&script);

    assert_eq!(client.workspaces(), Err(PostmanCloudError::RateLimited));
    assert!(pauses.lock().unwrap().is_empty());
}

#[test]
fn a_rejected_key_is_named() {
    let script = Script::default().answer(401, "{}");
    let (client, _) = client(&script);

    assert_eq!(client.workspaces(), Err(PostmanCloudError::KeyRejected));
}

#[test]
fn identifiers_that_would_change_the_path_are_refused() {
    let script = Script::default();
    let (client, _) = client(&script);

    assert!(matches!(
        client.collection("../me"),
        Err(PostmanCloudError::InvalidIdentifier(_))
    ));
    assert!(script.seen().is_empty());
}

#[test]
fn a_rejected_key_mid_way_skips_the_rest_and_says_why() {
    let script = Script::default().answer(200, WORKSPACE).answer(401, "{}");
    let (client, _) = client(&script);

    let report = migrate_workspace(&client, "w-1", &mut |_| {}).unwrap();

    assert_eq!(report.failures.len(), 2);
    assert!(report.failures[1].reason.contains("rejected"));
    assert_eq!(script.seen().len(), 2);
}

#[test]
fn a_missing_workspace_fails_the_whole_move() {
    let script = Script::default().answer(404, "{}");
    let (client, _) = client(&script);

    assert_eq!(
        migrate_workspace(&client, "w-1", &mut |_| {}),
        Err(PostmanCloudError::NotFound)
    );
}

#[test]
fn an_unreadable_retry_after_falls_back_to_the_next_header() {
    let wait = vec![
        Header::new("Retry-After", "Wed, 21 Oct 2026 07:28:00 GMT"),
        Header::new("X-RateLimit-RetryAfter", "3"),
    ];
    let script = Script::default()
        .answer_with(429, wait, "{}")
        .answer(200, WORKSPACES);
    let (client, pauses) = client(&script);

    client.workspaces().unwrap();
    assert_eq!(*pauses.lock().unwrap(), vec![Duration::from_secs(3)]);
}
