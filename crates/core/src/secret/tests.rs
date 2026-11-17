use super::{is_secret_name, Redactor, Secret, REDACTED};
use crate::collection::Auth;
use crate::environment::Environment;
use crate::variables::VariableScope;

fn scope_with_secrets() -> VariableScope {
    let environment = Environment::from_json(
        r#"{"name":"E","values":[
            {"key":"apiToken","value":"tok-123","type":"secret"},
            {"key":"base","value":"http://h","type":"default"}]}"#,
    )
    .unwrap();
    VariableScope::layered(&[], &[], Some(&environment), &[])
}

#[test]
fn secret_values_are_masked_in_any_query_parameter() {
    let redactor = Redactor::for_request(&scope_with_secrets(), None);
    assert_eq!(
        redactor.redact_url("http://h/p?q=tok-123&page=2"),
        format!("http://h/p?q={REDACTED}&page=2")
    );
}

#[test]
fn secret_values_are_masked_in_the_path_and_fragment() {
    let redactor = Redactor::for_request(&scope_with_secrets(), None);
    assert_eq!(
        redactor.redact_url("http://h/v1/tok-123/x#tok-123"),
        format!("http://h/v1/{REDACTED}/x#{REDACTED}")
    );
}

#[test]
fn auth_values_are_masked_even_when_literal_and_percent_encoded() {
    let auth = Auth::api_key("k", "a b&c", "query");
    let redactor = Redactor::for_request(&VariableScope::new(), Some(&auth));
    assert_eq!(
        redactor.redact_url("http://h/p?id=a%20b%26c"),
        format!("http://h/p?id={REDACTED}")
    );
    assert_eq!(redactor.redact_text("x a b&c y"), format!("x {REDACTED} y"));
}

#[test]
fn userinfo_is_masked_with_and_without_a_scheme() {
    let redactor = Redactor::default();
    assert_eq!(
        redactor.redact_url("https://user:pw@h:8080/a"),
        format!("https://{REDACTED}@h:8080/a")
    );
    assert_eq!(
        redactor.redact_url("user:pw@host/p"),
        format!("{REDACTED}@host/p")
    );
    assert_eq!(redactor.redact_url("host/p@x"), "host/p@x");
}

#[test]
fn secret_named_query_values_are_masked_and_others_kept() {
    let redactor = Redactor::default();
    assert_eq!(
        redactor.redact_url("http://h/p?page=2&api_key=abc&Token=x#top"),
        format!("http://h/p?page=2&api_key={REDACTED}&Token={REDACTED}#top")
    );
}

#[test]
fn debug_output_hides_values() {
    let redactor = Redactor::for_request(&scope_with_secrets(), None);
    assert!(!format!("{redactor:?}").contains("tok-123"));
}

#[test]
fn recognizes_secret_bearing_names() {
    for name in [
        "Authorization",
        "Cookie",
        "X-API-Key",
        "access_token",
        "client_secret",
    ] {
        assert!(is_secret_name(name), "{name}");
    }
    for name in ["Accept", "Content-Type", "page"] {
        assert!(!is_secret_name(name), "{name}");
    }
}

#[test]
fn debug_output_never_contains_the_value() {
    let secret = Secret::new("hunter2");
    assert!(!format!("{secret:?}").contains("hunter2"));
    assert_eq!(secret.expose(), "hunter2");
}

#[test]
fn values_shorter_than_the_minimum_are_not_substring_masked() {
    let environment = Environment::from_json(
        r#"{"name":"E","values":[{"key":"pin","value":"ab","type":"secret"}]}"#,
    )
    .unwrap();
    let scope = VariableScope::layered(&[], &[], Some(&environment), &[]);
    let redactor = Redactor::for_request(&scope, None);
    assert_eq!(redactor.redact_text("a b abc ab"), "a b abc ab");
}
