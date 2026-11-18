use std::time::Duration;

use requestor_core::checks::{ExtractSource, Extracted, Extraction, VariableTarget};
use requestor_core::collection::{Item, Request, RunStepOutcome, RunStepResult, Url};
use requestor_core::environment::Environment;

use super::state_with_request;
use crate::state::{Action, AppState, Effect, Overlay, RunnerForm};

fn runner(state: &mut AppState) -> &mut RunnerForm {
    match state.overlay.as_mut() {
        Some(Overlay::CollectionRunner(form)) => form,
        other => panic!("expected the runner, found {other:?}"),
    }
}

fn outcome(name: &str) -> RunStepOutcome {
    RunStepOutcome {
        result: RunStepResult {
            iteration: 1,
            name: name.to_string(),
            method: "GET".to_string(),
            status: Some(200),
            elapsed: Duration::from_millis(3),
            checks_passed: 0,
            checks_total: 0,
            error: None,
        },
        extracted: Vec::new(),
    }
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
            outcome: Box::new(outcome("create")),
        },
        0.0,
    );
    state.apply(
        Action::RunStepFinished {
            job: job + 7,
            outcome: Box::new(outcome("stale")),
        },
        0.0,
    );
    assert_eq!(runner(&mut state).results().len(), 1);
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

#[test]
fn leaving_the_runner_cancels_its_active_job() {
    let (mut state, collection_id, _) = state_with_request();
    for replacement in [Action::Show(Overlay::Settings), Action::CloseRunner] {
        state.apply(Action::OpenRunner(collection_id.clone()), 0.0);
        let job = start(&mut state);
        let effects = state.apply(replacement, 0.0);
        assert!(matches!(effects.as_slice(), [Effect::AbortSend(stopped)] if *stopped == job));
    }
}

#[test]
fn a_protected_run_requires_confirmation_for_the_selected_environment() {
    let (mut state, collection_id, _) = state_with_request();
    let first = state
        .workspace
        .add_environment(Environment::new("Production"));
    let second = state.workspace.add_environment(Environment::new("Prod"));
    state.apply(Action::SelectEnvironment(Some(first)), 0.0);
    state.apply(Action::OpenRunner(collection_id), 0.0);
    assert!(state.apply(Action::StartRun, 0.0).is_empty());
    assert!(runner(&mut state).is_confirming());

    state.apply(Action::SelectEnvironment(Some(second)), 0.0);
    assert!(state.apply(Action::StartRun, 0.0).is_empty());
    start(&mut state);
    assert!(!runner(&mut state).is_confirming());
}

#[test]
fn a_runner_failure_leaves_the_error_visible_and_allows_retry() {
    let (mut state, collection_id, _) = state_with_request();
    state.apply(Action::OpenRunner(collection_id), 0.0);
    let job = start(&mut state);
    let error = "invalid proxy";
    state.apply(
        Action::RunFinished {
            job,
            error: Some(error.to_string()),
        },
        0.0,
    );
    assert_eq!(runner(&mut state).failure(), Some(error));
    assert!(runner(&mut state).can_start());
    start(&mut state);
    assert_eq!(runner(&mut state).failure(), None);
}

#[test]
fn a_completed_step_stores_variables_in_its_original_environment_and_collection() {
    let (mut state, collection_id, _) = state_with_request();
    let dev = state.workspace.add_environment(Environment::new("Dev"));
    let staging = state.workspace.add_environment(Environment::new("Staging"));
    state.apply(Action::SelectEnvironment(Some(dev.clone())), 0.0);
    state.apply(Action::OpenRunner(collection_id.clone()), 0.0);
    let job = start(&mut state);
    state.apply(Action::SelectEnvironment(Some(staging.clone())), 0.0);

    let mut step = outcome("list");
    step.extracted = vec![
        extracted(VariableTarget::Environment),
        extracted(VariableTarget::Collection),
    ];
    state.apply(
        Action::RunStepFinished {
            job,
            outcome: Box::new(step),
        },
        0.0,
    );
    assert_eq!(
        state
            .workspace
            .environment(&dev)
            .unwrap()
            .value_of("id")
            .as_deref(),
        Some("42")
    );
    assert_eq!(
        state
            .workspace
            .environment(&staging)
            .unwrap()
            .value_of("id"),
        None
    );
    let collection = state.workspace.collection(&collection_id).unwrap();
    assert_eq!(collection.variable.as_ref().unwrap()[0].value_text(), "42");
    assert_eq!(runner(&mut state).passed_count(), 1);
}

fn start(state: &mut AppState) -> u64 {
    let effects = state.apply(Action::StartRun, 0.0);
    let [Effect::RunCollection(run)] = effects.as_slice() else {
        panic!("expected a run, got {effects:?}");
    };
    run.job
}

fn extracted(target: VariableTarget) -> Extracted {
    Extracted {
        extraction: Extraction {
            variable: "id".to_string(),
            source: ExtractSource::Header("X-Id".to_string()),
            target,
        },
        value: Ok("42".to_string()),
    }
}
