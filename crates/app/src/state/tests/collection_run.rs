use std::time::Duration;

use requestor_core::collection::{Item, Request, RunStepResult, Url};

use super::state_with_request;
use crate::state::{Action, AppState, Effect, Overlay, RunnerForm};

fn runner(state: &mut AppState) -> &mut RunnerForm {
    match state.overlay.as_mut() {
        Some(Overlay::CollectionRunner(form)) => form,
        other => panic!("expected the runner, found {other:?}"),
    }
}

fn result(name: &str) -> Box<RunStepResult> {
    Box::new(RunStepResult {
        iteration: 1,
        name: name.to_string(),
        method: "GET".to_string(),
        status: Some(200),
        elapsed: Duration::from_millis(3),
        checks_passed: 0,
        checks_total: 0,
        error: None,
    })
}

#[test]
fn a_run_sends_the_chosen_requests_in_the_chosen_order() {
    let (mut state, collection_id, _) = state_with_request();
    let request = Request::new("POST", Url::Raw("https://example.test/orders".to_string()));
    let collection = state.workspace.collection_mut(&collection_id).unwrap();
    collection.add_item(None, Item::for_request("create".to_string(), request));
    state.apply(Action::OpenRunner(collection_id), 0.0);
    let form = runner(&mut state);
    assert_eq!(form.steps.len(), 2);
    form.move_step(1, true);
    form.iterations_text = "3".to_string();
    form.steps[1].enabled = false;
    let effects = state.apply(Action::StartRun, 0.0);
    let [Effect::RunCollection(run)] = effects.as_slice() else {
        panic!("expected a run, got {effects:?}");
    };
    let names: Vec<_> = run
        .requests
        .iter()
        .map(|request| request.name.as_str())
        .collect();
    assert_eq!(names, ["create"]);
    assert_eq!(run.pacing.iterations(), 3);
    let job = run.job;
    assert!(runner(&mut state).is_running());
    assert_eq!(runner(&mut state).progress(), (0, 3));
    state.apply(
        Action::RunStepFinished {
            job,
            result: result("create"),
        },
        0.0,
    );
    state.apply(
        Action::RunStepFinished {
            job: job + 7,
            result: result("stale"),
        },
        0.0,
    );
    assert_eq!(runner(&mut state).results.len(), 1);
    state.apply(Action::RunFinished { job, error: None }, 0.0);
    assert!(!runner(&mut state).is_running());
}

#[test]
fn stopping_a_run_cancels_it() {
    let (mut state, collection_id, _) = state_with_request();
    state.apply(Action::OpenRunner(collection_id), 0.0);
    let effects = state.apply(Action::StartRun, 0.0);
    let [Effect::RunCollection(run)] = effects.as_slice() else {
        panic!("expected a run");
    };
    let job = run.job;
    let effects = state.apply(Action::StopRun(job), 0.0);
    assert!(matches!(effects.as_slice(), [Effect::AbortSend(stopped)] if *stopped == job));
    assert!(!runner(&mut state).is_running());
    assert!(state.apply(Action::StartRun, 0.0).len() == 1);
}
