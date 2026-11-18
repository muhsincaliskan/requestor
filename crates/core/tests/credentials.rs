use requestor_core::collection::{Auth, Collection, Item, Param, Request, Url, Variable};
use requestor_core::environment::Environment;

fn collection_with_credentials() -> Collection {
    let mut collection = Collection::new("Shop");
    collection.auth = Some(Auth::bearer("live-token"));
    collection.variable = Some(vec![
        Variable::text("apiKey", "abc123"),
        Variable::text("baseUrl", "https://shop.test"),
    ]);
    let mut request = Request::new("GET", Url::Raw("https://shop.test".to_string()));
    request.auth = Some(Auth::basic("user", "{{password}}"));
    request.header = Some(vec![
        Param::text("X-Api-Key", "header-secret"),
        Param::text("Accept", "application/json"),
    ]);
    collection.add_item(None, Item::for_request("List".to_string(), request));
    collection
}

#[test]
fn literal_credentials_are_counted_but_variable_references_are_not() {
    assert_eq!(collection_with_credentials().literal_secret_count(), 3);
}

#[test]
fn stripping_clears_only_literal_credentials() {
    let stripped = collection_with_credentials().without_literal_secrets();
    assert_eq!(stripped.literal_secret_count(), 0);
    let json = stripped.to_export_json().unwrap();
    for secret in ["live-token", "abc123", "header-secret"] {
        assert!(!json.contains(secret), "{secret} leaked");
    }
    for kept in [
        "{{password}}",
        "https://shop.test",
        "application/json",
        "X-Api-Key",
        "user",
    ] {
        assert!(json.contains(kept), "{kept} missing");
    }
}

#[test]
fn environments_report_and_strip_unmarked_secrets() {
    let mut environment = Environment::new("Staging");
    environment.set_value("authToken", "plain");
    environment.set_value("host", "staging.test");
    environment.set_value("password", "{{vault}}");
    assert_eq!(environment.unmarked_secret_count(), 1);
    let stripped = environment.without_unmarked_secrets();
    assert_eq!(stripped.value_of("authToken").as_deref(), Some(""));
    assert_eq!(stripped.value_of("host").as_deref(), Some("staging.test"));
}
