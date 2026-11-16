use requestor_core::import::{import_text, ImportError, Imported};
use requestor_core::prepared::PreparedRequest;
use requestor_core::settings::RequestSettings;
use requestor_core::variables::VariableScope;

const COLLECTION_FIXTURE: &str = include_str!("fixtures/collection.json");
const ENVIRONMENT_FIXTURE: &str = include_str!("fixtures/environment.json");

fn quiet_settings() -> RequestSettings {
    RequestSettings {
        send_no_cache_header: false,
        send_postman_token_header: false,
        ..RequestSettings::default()
    }
}

#[test]
fn detects_postman_collections_and_environments() {
    match import_text(COLLECTION_FIXTURE).unwrap() {
        Imported::Collection(collection) => assert_eq!(collection.request_count(), 9),
        other => panic!("expected a collection, got {other:?}"),
    }
    match import_text(ENVIRONMENT_FIXTURE).unwrap() {
        Imported::Environment(environment) => assert_eq!(environment.name, "Staging"),
        other => panic!("expected an environment, got {other:?}"),
    }
}

#[test]
fn detects_curl_commands_bare_urls_and_raw_requests() {
    let curl = import_text("  curl -X DELETE https://x.test/items/1  ").unwrap();
    let bare = import_text("https://x.test/health").unwrap();
    let raw = import_text("GET /ping HTTP/1.1\nHost: x.test").unwrap();
    let methods: Vec<_> = [curl, bare, raw]
        .into_iter()
        .map(|imported| match imported {
            Imported::Request(request) => request.request.method.unwrap(),
            other => panic!("expected a request, got {other:?}"),
        })
        .collect();
    assert_eq!(methods, ["DELETE", "GET", "GET"]);
}

#[test]
fn rejects_unknown_documents_with_clear_errors() {
    assert_eq!(import_text("   ").unwrap_err(), ImportError::Empty);
    assert!(matches!(
        import_text("{\"hello\": 1}").unwrap_err(),
        ImportError::UnsupportedFormat(_)
    ));
    assert!(matches!(
        import_text("{ not json").unwrap_err(),
        ImportError::InvalidJson(_)
    ));
    let yaml = import_text("openapi: 3.0.0\ninfo:\n  title: x").unwrap_err();
    assert!(yaml.to_string().contains("YAML"));
    assert!(matches!(
        import_text("just some words").unwrap_err(),
        ImportError::InvalidRequestLine(_)
    ));
}

#[test]
fn curl_import_survives_a_round_trip_through_request_preparation_and_export() {
    let command = "curl -X PUT 'https://api.test/v1/items?id=7' -H 'Content-Type: application/json' -H 'X-Note: it'\\''s' -d '{\"a\":1}'";
    let Imported::Request(imported) = import_text(command).unwrap() else {
        panic!("expected a request");
    };
    let prepared =
        PreparedRequest::from_request(&imported.request, &VariableScope::new(), &quiet_settings())
            .unwrap();
    let exported = prepared.to_curl();
    assert!(exported.contains("-X PUT"));
    assert!(exported.contains("'https://api.test/v1/items?id=7'"));
    assert!(exported.contains("-H 'X-Note: it'\\''s'"));
    let Imported::Request(again) = import_text(&exported).unwrap() else {
        panic!("expected a request");
    };
    let reprepared =
        PreparedRequest::from_request(&again.request, &VariableScope::new(), &quiet_settings())
            .unwrap();
    assert_eq!(reprepared.url, prepared.url);
    assert_eq!(reprepared.body, prepared.body);
    assert_eq!(reprepared.header("x-note"), Some("it's"));
}
