use requestor_core::graphql::GraphQlSchema;
use requestor_core::method::HttpMethod;

use super::{open, state_with_request};
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

fn fetch(state: &mut AppState, tab: TabId) -> (u64, Vec<Effect>) {
    let effects = state.apply(Action::FetchSchema(tab), 0.0);
    let job = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::FetchSchema { job, request, .. } => {
                assert_eq!(request.method, HttpMethod::Post);
                Some(*job)
            }
            _ => None,
        })
        .expect("a schema request");
    (job, effects)
}

fn answer(state: &mut AppState, tab: TabId, job: u64, result: Result<GraphQlSchema, String>) {
    state.apply(Action::SchemaFetched { tab, job, result }, 0.0);
}

fn parsed() -> GraphQlSchema {
    GraphQlSchema::from_response(SCHEMA.as_bytes()).unwrap()
}

fn aborted(effects: &[Effect], job: u64) -> bool {
    effects
        .iter()
        .any(|effect| matches!(effect, Effect::AbortSend(stopped) if *stopped == job))
}

#[test]
fn a_fetched_schema_is_kept_on_the_tab() {
    let (mut state, tab) = opened();
    let (job, _) = fetch(&mut state, tab);
    assert!(state.tab(tab).unwrap().schema.is_fetching());
    answer(&mut state, tab, job, Ok(parsed()));
    let SchemaLookup::Ready(schema) = &state.tab(tab).unwrap().schema.lookup else {
        panic!("expected a schema");
    };
    assert_eq!(schema.type_count(), 2);
}

#[test]
fn fetching_again_stops_the_older_request_and_ignores_its_answer() {
    let (mut state, tab) = opened();
    let (first, _) = fetch(&mut state, tab);
    let (second, effects) = fetch(&mut state, tab);
    assert!(aborted(&effects, first));
    answer(&mut state, tab, first, Ok(parsed()));
    assert!(state.tab(tab).unwrap().schema.is_fetching());
    answer(&mut state, tab, second, Err("timed out".to_string()));
    assert!(matches!(
        &state.tab(tab).unwrap().schema.lookup,
        SchemaLookup::Failed(reason) if reason == "timed out"
    ));
}

#[test]
fn cancelling_or_closing_the_tab_stops_the_request() {
    let (mut state, tab) = opened();
    let (job, _) = fetch(&mut state, tab);
    let effects = state.apply(Action::CancelSchema(tab), 0.0);
    assert!(aborted(&effects, job));
    assert!(matches!(
        state.tab(tab).unwrap().schema.lookup,
        SchemaLookup::Idle
    ));
    let (job, _) = fetch(&mut state, tab);
    let effects = state.apply(Action::ForceCloseTab(tab), 0.0);
    assert!(aborted(&effects, job));
    answer(&mut state, tab, job, Ok(parsed()));
    assert!(state.tab(tab).is_none());
}
