use requestor_core::collection::{
    BodyMode, Collection, Item, Request, Url, POSTMAN_COLLECTION_SCHEMA_V21,
};
use requestor_core::environment::Environment;
use requestor_core::method::HttpMethod;
use serde_json::Value;

const COLLECTION_FIXTURE: &str = include_str!("fixtures/collection.json");
const ENVIRONMENT_FIXTURE: &str = include_str!("fixtures/environment.json");

fn as_value(text: &str) -> Value {
    serde_json::from_str(text).expect("fixture is valid JSON")
}

#[test]
fn collection_round_trips_without_losing_fields() {
    let collection = Collection::from_json(COLLECTION_FIXTURE).unwrap();
    let exported = collection.to_json_pretty().unwrap();
    assert_eq!(as_value(&exported), as_value(COLLECTION_FIXTURE));
}

#[test]
fn environment_round_trips_without_losing_fields() {
    let environment = Environment::from_json(ENVIRONMENT_FIXTURE).unwrap();
    let exported = environment.to_json_pretty().unwrap();
    assert_eq!(as_value(&exported), as_value(ENVIRONMENT_FIXTURE));
}

#[test]
fn counts_requests_across_nested_folders() {
    let collection = Collection::from_json(COLLECTION_FIXTURE).unwrap();
    assert_eq!(collection.request_count(), 7);
    assert!(collection.item[0].is_folder());
    assert!(!collection.item[1].is_folder());
}

#[test]
fn reads_url_as_string_or_object() {
    let collection = Collection::from_json(COLLECTION_FIXTURE).unwrap();
    let folder = collection.item[0].item.as_ref().unwrap();
    let create = request_of(&folder[1]);
    let list = request_of(&folder[0]);
    assert!(matches!(create.url, Some(Url::Raw(_))));
    assert_eq!(
        list.url.as_ref().unwrap().to_text(),
        "{{baseUrl}}/users?page=1&limit=20"
    );
}

#[test]
fn reads_body_modes_and_methods() {
    let collection = Collection::from_json(COLLECTION_FIXTURE).unwrap();
    let login = request_of(&collection.item[1]);
    let purge = request_of(&collection.item[4]);
    assert_eq!(
        login.body.as_ref().unwrap().mode,
        Some(BodyMode::Urlencoded)
    );
    assert_eq!(login.body.as_ref().unwrap().enabled_form_params().len(), 2);
    assert_eq!(purge.http_method(), HttpMethod::Custom("PURGE".to_string()));
}

#[test]
fn skips_disabled_headers_and_form_params() {
    let collection = Collection::from_json(COLLECTION_FIXTURE).unwrap();
    let folder = collection.item[0].item.as_ref().unwrap();
    let create = request_of(&folder[1]);
    let upload = request_of(&folder[2].item.as_ref().unwrap()[0]);
    assert_eq!(create.enabled_headers().len(), 1);
    assert_eq!(upload.body.as_ref().unwrap().enabled_form_params().len(), 2);
}

#[test]
fn exposes_auth_values_and_collection_variables() {
    let collection = Collection::from_json(COLLECTION_FIXTURE).unwrap();
    let auth = collection.auth.as_ref().unwrap();
    assert_eq!(auth.value_of("username"), Some("ada"));
    let variables = collection.variable.as_ref().unwrap();
    assert_eq!(variables[1].text_value().as_deref(), Some("3"));
}

#[test]
fn preserves_unknown_fields() {
    let collection = Collection::from_json(COLLECTION_FIXTURE).unwrap();
    assert!(collection.extra.contains_key("protocolProfileBehavior"));
    assert!(collection.info.extra.contains_key("_exporter_id"));
}

#[test]
fn environment_ignores_disabled_values() {
    let environment = Environment::from_json(ENVIRONMENT_FIXTURE).unwrap();
    assert_eq!(
        environment.value_of("baseUrl"),
        Some("https://staging.example.com")
    );
    assert_eq!(environment.value_of("legacy"), None);
}

#[test]
fn new_collection_uses_postman_v21_schema() {
    let collection = Collection::new("Empty");
    assert_eq!(
        collection.info.schema.as_deref(),
        Some(POSTMAN_COLLECTION_SCHEMA_V21)
    );
}

fn request_of(item: &Item) -> Request {
    item.request
        .as_ref()
        .expect("item has a request")
        .to_request()
}
