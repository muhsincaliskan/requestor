use requestor_core::collection::Collection;
use requestor_core::migration::MigrationReport;
use requestor_core::postman_cloud::{CloudWorkspace, MigrationProgress};
use requestor_core::secret::Secret;
use requestor_core::workspace::Workspace;

use super::state_with;
use crate::state::{Action, AppState, Effect, Overlay};

const KEY: &str = "PMAK-123";

fn workspace(id: &str) -> CloudWorkspace {
    CloudWorkspace {
        id: id.to_string(),
        name: format!("Workspace {id}"),
        kind: "personal".to_string(),
    }
}

fn listed_state() -> AppState {
    let mut state = state_with(Workspace::default());
    state.postman.api_key = format!("  {KEY} ");
    let effects = state.apply(Action::ListPostmanWorkspaces, 0.0);
    let job = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::ListPostmanWorkspaces { job, .. } => Some(*job),
            _ => None,
        })
        .expect("listing starts");
    state.apply(
        Action::PostmanWorkspacesListed {
            job,
            listed: Ok(vec![workspace("a"), workspace("b")]),
        },
        0.0,
    );
    state
}

#[test]
fn opening_the_dialog_looks_up_a_saved_key_once() {
    let mut state = state_with(Workspace::default());
    let effects = state.apply(Action::OpenPostmanMigration, 0.0);
    assert!(matches!(effects.as_slice(), [Effect::LoadPostmanKey]));
    assert_eq!(state.overlay, Some(Overlay::PostmanMigration));
    state.apply(Action::PostmanKeyLoaded(Some(Secret::new(KEY))), 0.0);
    assert_eq!(state.postman.api_key, KEY);

    assert!(state.apply(Action::OpenPostmanMigration, 0.0).is_empty());
}

#[test]
fn listing_keeps_or_forgets_the_key_as_asked_and_picks_the_first_workspace() {
    let state = listed_state();
    assert_eq!(state.postman.selected.as_deref(), Some("a"));
    assert_eq!(state.postman.workspaces.len(), 2);

    let mut state = state_with(Workspace::default());
    state.postman.api_key = KEY.to_string();
    state.postman.remember_key = false;
    let effects = state.apply(Action::ListPostmanWorkspaces, 0.0);
    assert!(matches!(effects[0], Effect::StorePostmanKey(None)));
    let Effect::ListPostmanWorkspaces { key, .. } = &effects[1] else {
        panic!("listing starts");
    };
    assert_eq!(key.expose(), KEY);
}

#[test]
fn a_rejected_key_shows_on_the_form() {
    let mut state = state_with(Workspace::default());
    state.postman.api_key = KEY.to_string();
    state.apply(Action::ListPostmanWorkspaces, 0.0);
    state.apply(
        Action::PostmanWorkspacesListed {
            job: 1,
            listed: Err("Postman rejected the API key".to_string()),
        },
        0.0,
    );
    assert!(!state.postman.is_listing());
    assert!(state.postman.error.is_some());
}

#[test]
fn an_empty_key_lists_nothing() {
    let mut state = state_with(Workspace::default());
    state.postman.api_key = "   ".to_string();
    assert!(state.apply(Action::ListPostmanWorkspaces, 0.0).is_empty());
}

#[test]
fn moving_a_workspace_shows_progress_and_imports_the_result() {
    let mut state = listed_state();
    state.postman.selected = Some("b".to_string());
    let effects = state.apply(Action::MigratePostmanWorkspace, 0.0);
    let job = match effects.as_slice() {
        [Effect::MigratePostmanWorkspace {
            job, workspace_id, ..
        }] => {
            assert_eq!(workspace_id, "b");
            *job
        }
        other => panic!("expected a migration, got {other:?}"),
    };
    let progress = MigrationProgress {
        done: 0,
        total: 2,
        current: "Orders".to_string(),
    };
    state.apply(Action::MigrationProgressed { job, progress }, 0.0);
    assert!(state.postman.progress.is_some());

    let mut report = MigrationReport::default();
    report.add_collection(Collection::new("Orders"));
    state.apply(
        Action::MigrationFinished {
            job,
            report: Box::new(report),
        },
        0.0,
    );
    assert_eq!(state.workspace.collections.len(), 1);
    assert!(state.postman.progress.is_none());
}

#[test]
fn the_form_never_prints_the_key() {
    let state = listed_state();
    assert!(!format!("{:?}", state.postman).contains(KEY));
}
