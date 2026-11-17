use requestor_core::draft::{CheckKind, CheckRow, ExtractKind, ExtractRow};
use requestor_core::environment::Environment;

use super::{open, response, state_with_request, stored_url};
use crate::state::{Action, Effect, Exchange};

fn rules_tab_sent(body: &[u8]) -> (crate::state::AppState, String, String, String) {
    let (mut state, collection_id, item_id) = state_with_request();
    let environment_id = state.workspace.add_environment(Environment::new("Dev"));
    state.workspace.session.active_environment = Some(environment_id.clone());
    let tab = open(&mut state, &collection_id, &item_id);
    let rules = &mut state.tab_mut(tab).unwrap().draft.rules;
    rules.checks.push(CheckRow {
        kind: CheckKind::Status,
        subject: String::new(),
        expected: "200".to_string(),
    });
    rules.extractions.push(ExtractRow {
        variable: "token".to_string(),
        kind: ExtractKind::Json,
        source: "auth.token".to_string(),
        target: Default::default(),
    });
    let effects = state.apply(Action::Send(tab), 1.0);
    let Some(Effect::Send { job, .. }) = effects.first() else {
        panic!("no send effect");
    };
    state.apply(
        Action::SendFinished {
            tab,
            job: *job,
            result: Ok(response(body)),
        },
        2.0,
    );
    (state, collection_id, item_id, environment_id)
}

#[test]
fn a_finished_send_runs_the_checks_and_stores_extracted_values() {
    let (state, _, _, environment_id) = rules_tab_sent(br#"{"auth":{"token":"t-1"}}"#);
    let tab = state.active().unwrap();
    let Exchange::Completed(view) = &tab.exchange else {
        panic!("not completed");
    };
    let report = view.report.as_ref().unwrap();
    assert!(report.all_passed());
    let environment = state.workspace.environment(&environment_id).unwrap();
    assert_eq!(environment.value_of("token").as_deref(), Some("t-1"));
}

#[test]
fn saving_a_tab_writes_its_rules_as_a_postman_test_event() {
    let (mut state, collection_id, item_id, _) = rules_tab_sent(b"{}");
    let tab = state.active_tab.unwrap();
    state.apply(Action::SaveTab(tab), 3.0);
    let collection = state.workspace.collection(&collection_id).unwrap();
    let item = collection.find_item(&item_id).unwrap();
    let event = &item.event.as_ref().unwrap()[0];
    assert_eq!(event.listen, "test");
    assert!(!state.tab(tab).unwrap().unsaved);
    assert_eq!(
        stored_url(&state, &collection_id, &item_id),
        "https://example.test/users"
    );
}
