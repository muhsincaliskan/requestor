use super::{BodyKind, KeyValueRow, RequestDraft};
use crate::collection::{Collection, Request, Url};
use crate::prepared::PreparedRequest;
use crate::settings::RequestSettings;
use crate::variables::{VariableLayer, VariableScope};

const COLLECTION_FIXTURE: &str = include_str!("../../tests/fixtures/collection.json");

fn draft_of(json: &str) -> (Request, RequestDraft) {
    let request: Request = serde_json::from_str(json).unwrap();
    let draft = RequestDraft::from_request("r", &request);
    (request, draft)
}

fn every_request(items: &[crate::collection::Item], found: &mut Vec<Request>) {
    for item in items {
        found.extend(item.full_request());
        every_request(item.children(), found);
    }
}

#[test]
fn untouched_drafts_return_the_stored_request_for_every_fixture_request() {
    let collection = Collection::from_json(COLLECTION_FIXTURE).unwrap();
    let mut requests = Vec::new();
    every_request(&collection.item, &mut requests);
    assert!(!requests.is_empty());
    for request in requests {
        let draft = RequestDraft::from_request("r", &request);
        assert!(draft.is_unchanged());
        assert_eq!(draft.to_request(), request);
    }
}

#[test]
fn edits_keep_unknown_fields_of_the_request_and_its_url() {
    let (_, mut draft) = draft_of(
        r#"{"method":"GET","url":{"raw":"https://x.test/a?b=1","host":["x","test"],"path":["a"],
            "query":[{"key":"b","value":"1"}],"x-url":1},"x-request":true}"#,
    );
    draft.method = "post".to_string();
    let request = draft.to_request();
    assert_eq!(request.method.as_deref(), Some("POST"));
    assert!(request.extra.contains_key("x-request"));
    match request.url.unwrap() {
        Url::Parts(parts) => assert!(parts.extra.contains_key("x-url")),
        Url::Raw(_) => panic!("expected url parts"),
    }
}

#[test]
fn editing_the_url_rebuilds_the_query_table_and_keeps_disabled_rows() {
    let (_, mut draft) = draft_of(
        r#"{"url":{"raw":"https://x.test/a?b=1","query":[
            {"key":"b","value":"1","description":"first"},
            {"key":"off","value":"0","disabled":true}]}}"#,
    );
    draft.url = "https://x.test/a?b=2&c=3&flag#top".to_string();
    draft.url_edited();
    let rows: Vec<_> = draft
        .query
        .iter()
        .map(|row| (row.key.as_str(), row.value.as_str(), row.enabled))
        .collect();
    assert_eq!(
        rows,
        [
            ("b", "2", true),
            ("c", "3", true),
            ("flag", "", true),
            ("off", "0", false)
        ]
    );
    assert_eq!(draft.query[0].description, "first");
}

#[test]
fn editing_the_query_table_rebuilds_the_url_without_disabled_or_blank_rows() {
    let mut draft = RequestDraft::blank("r");
    draft.url = "https://x.test/a?old=1#top".to_string();
    draft.url_edited();
    draft.query[0].value = "2".to_string();
    draft.query.push(KeyValueRow::new("q", "a b"));
    draft.query.push(KeyValueRow::default());
    let mut disabled = KeyValueRow::new("off", "1");
    disabled.enabled = false;
    draft.query.push(disabled);
    draft.query_edited();
    assert_eq!(draft.url, "https://x.test/a?old=2&q=a b#top");
    draft.query.clear();
    draft.query_edited();
    assert_eq!(draft.url, "https://x.test/a#top");
}

#[test]
fn path_variables_follow_the_url_and_keep_their_values() {
    let mut draft = RequestDraft::blank("r");
    draft.url = "{{baseUrl}}/users/:id".to_string();
    draft.url_edited();
    draft.path_variables[0].value = "42".to_string();
    draft.url = "{{baseUrl}}/users/:id/posts/:post".to_string();
    draft.url_edited();
    let rows: Vec<_> = draft
        .path_variables
        .iter()
        .map(|row| (row.key.as_str(), row.value.as_str()))
        .collect();
    assert_eq!(rows, [("id", "42"), ("post", "")]);
    draft.url = "{{baseUrl}}/users".to_string();
    draft.url_edited();
    assert!(draft.path_variables.is_empty());
}

#[test]
fn a_draft_built_from_scratch_prepares_into_the_expected_request() {
    let mut draft = RequestDraft::blank("create");
    draft.method = "POST".to_string();
    draft.url = "{{host}}/users/:id?verbose=1".to_string();
    draft.url_edited();
    draft.path_variables[0].value = "7".to_string();
    draft.headers.push(KeyValueRow::new("X-Trace", "{{trace}}"));
    draft.body.kind = BodyKind::Raw;
    draft.body.raw = r#"{"name":"ada"}"#.to_string();
    draft.body.language = super::RawLanguage::Json;
    draft.auth.kind = super::AuthKind::Bearer;
    draft.auth.token = "{{token}}".to_string();
    let scope = VariableScope::new().with_layer(
        VariableLayer::default()
            .with("host", "https://x.test")
            .with("trace", "t-1")
            .with("token", "secret"),
    );
    let settings = RequestSettings {
        send_no_cache_header: false,
        send_postman_token_header: false,
        ..RequestSettings::default()
    };
    let prepared = PreparedRequest::from_request(&draft.to_request(), &scope, &settings).unwrap();
    assert_eq!(prepared.url, "https://x.test/users/7?verbose=1");
    assert_eq!(prepared.header("X-Trace"), Some("t-1"));
    assert_eq!(prepared.header("Authorization"), Some("Bearer secret"));
    assert_eq!(prepared.header("Content-Type"), Some("application/json"));
    assert_eq!(
        prepared.body.as_deref(),
        Some(r#"{"name":"ada"}"#.as_bytes())
    );
}

#[test]
fn an_empty_url_is_stored_as_no_url() {
    let mut draft = RequestDraft::blank("r");
    draft.name = "renamed".to_string();
    draft.headers.push(KeyValueRow::new("A", "1"));
    assert!(draft.to_request().url.is_none());
}

#[test]
fn section_counts_include_only_enabled_non_blank_rows() {
    let mut draft = RequestDraft::blank("r");
    draft.url = "https://x.test/:id?a=1".to_string();
    draft.url_edited();
    draft.headers.push(KeyValueRow::new("A", "1"));
    draft.headers.push(KeyValueRow::default());
    assert_eq!(draft.active_param_count(), 2);
    assert_eq!(draft.active_header_count(), 1);
}

const EXTRA_FIELDS_REQUEST: &str = r#"{"method":"GET","url":{"raw":"https://x.test/u/:id?a=1",
    "host":["x","test"],"path":["u",":id"],
    "query":[{"key":"a","value":"1","x-q":true,"description":"keep"}],
    "variable":[{"id":"v1","key":"id","value":8080,"type":"any","name":"n","system":true,"x-v":1}]}}"#;

fn serialized(draft: &RequestDraft) -> serde_json::Value {
    serde_json::to_value(draft.to_request()).unwrap()
}

#[test]
fn an_unedited_draft_with_extra_fields_round_trips_exactly() {
    let (request, draft) = draft_of(EXTRA_FIELDS_REQUEST);
    assert_eq!(draft.to_request(), request);
}

#[test]
fn editing_a_path_variable_keeps_its_extra_fields_and_non_string_value() {
    let (_, mut draft) = draft_of(EXTRA_FIELDS_REQUEST);
    draft.method = "POST".to_string();
    let json = serialized(&draft);
    let variable = &json["url"]["variable"][0];
    assert_eq!(variable["id"], "v1");
    assert_eq!(variable["type"], "any");
    assert_eq!(variable["name"], "n");
    assert_eq!(variable["system"], true);
    assert_eq!(variable["x-v"], 1);
    assert_eq!(variable["value"], 8080);
    draft.path_variables[0].value = "9".to_string();
    let json = serialized(&draft);
    assert_eq!(json["url"]["variable"][0]["value"], "9");
    assert_eq!(json["url"]["variable"][0]["x-v"], 1);
}

#[test]
fn typing_in_the_url_keeps_query_row_extra_fields() {
    let (_, mut draft) = draft_of(EXTRA_FIELDS_REQUEST);
    draft.url = "https://x.test/u/:id?a=2".to_string();
    draft.url_edited();
    let json = serialized(&draft);
    let query = &json["url"]["query"][0];
    assert_eq!(query["value"], "2");
    assert_eq!(query["x-q"], true);
    assert_eq!(query["description"], "keep");
}

#[test]
fn editing_auth_through_the_draft_keeps_other_auth_sections() {
    let (_, mut draft) = draft_of(
        r#"{"method":"GET","auth":{"type":"bearer","bearer":[{"key":"token","value":"a","x-attr":1}],
            "oauth2":[{"key":"accessToken","value":"t"}]}}"#,
    );
    draft.auth.token = "b".to_string();
    let json = serialized(&draft);
    assert_eq!(json["auth"]["bearer"][0]["value"], "b");
    assert_eq!(json["auth"]["bearer"][0]["x-attr"], 1);
    assert_eq!(json["auth"]["oauth2"][0]["key"], "accessToken");
}

#[test]
fn query_value_presence_follows_the_typed_url_in_both_directions() {
    let (_, mut draft) = draft_of(
        r#"{"method":"GET","url":{"raw":"https://x.test/?flag&a=","query":[{"key":"flag"},{"key":"a","value":""}]}}"#,
    );
    for typed in ["https://x.test/?flag=&a", "https://x.test/?flag&a="] {
        draft.url = typed.to_string();
        draft.url_edited();
        draft.query_edited();
        assert_eq!(draft.url, typed);
    }
    let mut blank = RequestDraft::blank("r");
    blank.url = "https://x.test/?a=1".to_string();
    blank.url_edited();
    blank.url = "https://x.test/?a".to_string();
    blank.url_edited();
    blank.query_edited();
    assert_eq!(blank.url, "https://x.test/?a");
}
