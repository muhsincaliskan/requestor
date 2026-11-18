use requestor_core::method::HttpMethod;
use requestor_core::response::HttpResponse;

use super::{open, response, state_with_request};
use crate::state::{Action, AppState, Effect, SchemaLookup, TabId};

const SCHEMA: &str = r#"{"data":{"__schema":{"queryType":{"name":"Query"},"types":[
    {"kind":"OBJECT","name":"Query","fields":[
        {"name":"ping","args":[],"type":{"kind":"SCALAR","name":"String"}}]},
    {"kind":"SCALAR","name":"String"}]}}}"#;

fn opened() -> (AppState, TabId) {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    (state, tab)
}

fn fetch(state: &mut AppState, tab: TabId) -> u64 {
    let effects = state.apply(Action::FetchSchema(tab), 0.0);
    let [Effect::FetchSchema { job, request, .. }] = effects.as_slice() else {
        panic!("expected a schema request, got {effects:?}");
    };
    assert_eq!(request.method, HttpMethod::Post);
    *job
}

fn answer(state: &mut AppState, tab: TabId, job: u64, result: Result<HttpResponse, String>) {
    state.apply(Action::SchemaFetched { tab, job, result }, 0.0);
}

#[test]
fn a_fetched_schema_is_kept_on_the_tab() {
    let (mut state, tab) = opened();
    let job = fetch(&mut state, tab);
    assert!(state.tab(tab).unwrap().schema.is_fetching());
    answer(&mut state, tab, job, Ok(response(SCHEMA.as_bytes())));
    let SchemaLookup::Ready(schema) = &state.tab(tab).unwrap().schema.lookup else {
        panic!("expected a schema");
    };
    assert_eq!(schema.type_count(), 2);
}

#[test]
fn an_older_answer_is_ignored() {
    let (mut state, tab) = opened();
    let first = fetch(&mut state, tab);
    let second = fetch(&mut state, tab);
    answer(&mut state, tab, first, Ok(response(SCHEMA.as_bytes())));
    assert!(state.tab(tab).unwrap().schema.is_fetching());
    answer(&mut state, tab, second, Err("timed out".to_string()));
    assert!(matches!(
        &state.tab(tab).unwrap().schema.lookup,
        SchemaLookup::Failed(reason) if reason == "timed out"
    ));
}

#[test]
fn a_page_that_is_not_graphql_explains_the_status() {
    let (mut state, tab) = opened();
    let job = fetch(&mut state, tab);
    let mut missing = response(b"<html>not found</html>");
    missing.status = 404;
    answer(&mut state, tab, job, Ok(missing));
    let SchemaLookup::Failed(reason) = &state.tab(tab).unwrap().schema.lookup else {
        panic!("expected a failure");
    };
    assert!(reason.contains("404"), "{reason}");
}
