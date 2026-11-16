use requestor_core::collection::{Collection, Request};
use requestor_core::error::CoreError;
use requestor_core::method::HttpMethod;
use requestor_core::prepared::PreparedRequest;
use requestor_core::settings::RequestSettings;
use requestor_core::variables::{VariableLayer, VariableScope};
use serde_json::{json, Value};

fn request_from(request: Value) -> Request {
    let collection = json!({"info": {"name": "t"}, "item": [{"name": "r", "request": request}]});
    let collection = Collection::from_json(&collection.to_string()).unwrap();
    collection.item[0].request.as_ref().unwrap().to_request()
}

fn scope() -> VariableScope {
    VariableScope::new().with_layer(
        VariableLayer::default()
            .with("host", "api.example.com")
            .with("token", "abc123")
            .with("name", "Ada Lovelace"),
    )
}

fn quiet_settings() -> RequestSettings {
    RequestSettings {
        send_no_cache_header: false,
        send_postman_token_header: false,
        ..RequestSettings::default()
    }
}

fn prepare(request: Value) -> PreparedRequest {
    PreparedRequest::from_request(&request_from(request), &scope(), &quiet_settings()).unwrap()
}

#[test]
fn resolves_variables_in_url_and_headers() {
    let prepared = prepare(json!({
        "method": "POST",
        "header": [
            {"key": "X-Name", "value": "{{name}}"},
            {"key": "X-Skip", "value": "1", "disabled": true}
        ],
        "url": "https://{{host}}/users"
    }));
    assert_eq!(prepared.method, HttpMethod::Post);
    assert_eq!(prepared.url, "https://api.example.com/users");
    assert_eq!(prepared.header("x-name"), Some("Ada Lovelace"));
    assert_eq!(prepared.header("X-Skip"), None);
}

#[test]
fn missing_url_is_an_error() {
    let request = request_from(json!({"method": "GET"}));
    let result = PreparedRequest::from_request(&request, &scope(), &quiet_settings());
    assert!(matches!(result, Err(CoreError::MissingUrl)));
}

#[test]
fn raw_json_body_sets_content_type_and_resolves_variables() {
    let prepared = prepare(json!({
        "method": "POST",
        "url": "http://x/",
        "body": {"mode": "raw", "raw": "{\"n\": \"{{name}}\"}", "options": {"raw": {"language": "json"}}}
    }));
    assert_eq!(prepared.header("Content-Type"), Some("application/json"));
    assert_eq!(prepared.body.unwrap(), br#"{"n": "Ada Lovelace"}"#);
}

#[test]
fn explicit_content_type_header_is_not_overridden() {
    let prepared = prepare(json!({
        "method": "POST",
        "header": [{"key": "content-type", "value": "text/csv"}],
        "url": "http://x/",
        "body": {"mode": "raw", "raw": "a,b"}
    }));
    assert_eq!(prepared.header("Content-Type"), Some("text/csv"));
    assert_eq!(prepared.headers.len(), 1);
}

#[test]
fn urlencoded_body_escapes_and_skips_disabled() {
    let prepared = prepare(json!({
        "method": "POST",
        "url": "http://x/",
        "body": {"mode": "urlencoded", "urlencoded": [
            {"key": "name", "value": "{{name}}"},
            {"key": "q", "value": "a&b"},
            {"key": "off", "value": "1", "disabled": true}
        ]}
    }));
    assert_eq!(
        prepared.body.clone().unwrap(),
        b"name=Ada%20Lovelace&q=a%26b"
    );
    assert_eq!(
        prepared.header("Content-Type"),
        Some("application/x-www-form-urlencoded")
    );
}

#[test]
fn formdata_body_contains_text_and_file_parts() {
    let path = std::env::temp_dir().join("requestor-formdata-fixture.txt");
    std::fs::write(&path, "file-bytes").unwrap();
    let prepared = prepare(json!({
        "method": "POST",
        "url": "http://x/",
        "body": {"mode": "formdata", "formdata": [
            {"key": "title", "value": "{{name}}", "type": "text"},
            {"key": "upload", "type": "file", "src": path.to_string_lossy()}
        ]}
    }));
    let content_type = prepared.header("Content-Type").unwrap().to_string();
    let boundary = content_type.split("boundary=").nth(1).unwrap();
    let body = String::from_utf8(prepared.body.unwrap()).unwrap();
    assert!(body.contains("name=\"title\"\r\n\r\nAda Lovelace"));
    assert!(body.contains("name=\"upload\"; filename=\"requestor-formdata-fixture.txt\""));
    assert!(body.contains("file-bytes"));
    assert!(body.ends_with(&format!("--{boundary}--\r\n")));
}

#[test]
fn unreadable_file_body_reports_the_path() {
    let request = request_from(json!({
        "method": "POST",
        "url": "http://x/",
        "body": {"mode": "file", "file": {"src": "/definitely/not/here.bin"}}
    }));
    let error = PreparedRequest::from_request(&request, &scope(), &quiet_settings()).unwrap_err();
    assert!(error.to_string().contains("/definitely/not/here.bin"));
}

#[test]
fn graphql_body_becomes_json_payload() {
    let prepared = prepare(json!({
        "method": "POST",
        "url": "http://x/graphql",
        "body": {"mode": "graphql", "graphql": {"query": "{ user(name: \"{{name}}\") { id } }", "variables": "{\"a\": 1}"}}
    }));
    let payload: Value = serde_json::from_slice(&prepared.body.unwrap()).unwrap();
    assert_eq!(payload["query"], "{ user(name: \"Ada Lovelace\") { id } }");
    assert_eq!(payload["variables"], json!({"a": 1}));
}

#[test]
fn bearer_and_basic_auth_add_authorization_header() {
    let bearer = prepare(json!({
        "method": "GET", "url": "http://x/",
        "auth": {"type": "bearer", "bearer": [{"key": "token", "value": "{{token}}"}]}
    }));
    assert_eq!(bearer.header("Authorization"), Some("Bearer abc123"));
    let basic = prepare(json!({
        "method": "GET", "url": "http://x/",
        "auth": {"type": "basic", "basic": [
            {"key": "username", "value": "user"}, {"key": "password", "value": "pass"}
        ]}
    }));
    assert_eq!(basic.header("Authorization"), Some("Basic dXNlcjpwYXNz"));
}

#[test]
fn explicit_authorization_header_wins_over_auth() {
    let prepared = prepare(json!({
        "method": "GET", "url": "http://x/",
        "header": [{"key": "Authorization", "value": "Custom 1"}],
        "auth": {"type": "bearer", "bearer": [{"key": "token", "value": "abc"}]}
    }));
    assert_eq!(prepared.header("Authorization"), Some("Custom 1"));
}

#[test]
fn api_key_goes_to_header_or_query() {
    let header = prepare(json!({
        "method": "GET", "url": "http://x/",
        "auth": {"type": "apikey", "apikey": [
            {"key": "key", "value": "X-Key"}, {"key": "value", "value": "k1"}, {"key": "in", "value": "header"}
        ]}
    }));
    assert_eq!(header.header("X-Key"), Some("k1"));
    let query = prepare(json!({
        "method": "GET", "url": "http://x/p?a=1#frag",
        "auth": {"type": "apikey", "apikey": [
            {"key": "key", "value": "api key"}, {"key": "value", "value": "v&1"}, {"key": "in", "value": "query"}
        ]}
    }));
    assert_eq!(query.url, "http://x/p?a=1&api%20key=v%261#frag");
}

#[test]
fn default_headers_follow_settings() {
    let request = request_from(json!({"method": "GET", "url": "http://x/"}));
    let with =
        PreparedRequest::from_request(&request, &scope(), &RequestSettings::default()).unwrap();
    assert_eq!(with.header("Cache-Control"), Some("no-cache"));
    assert!(with.header("Postman-Token").is_some());
    let without = PreparedRequest::from_request(&request, &scope(), &quiet_settings()).unwrap();
    assert!(without.headers.is_empty());
}
