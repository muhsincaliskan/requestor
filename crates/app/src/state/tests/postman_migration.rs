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
fn listing_picks_the_first_workspace() {
    let state = listed_state();
    assert_eq!(state.postman.selected.as_deref(), Some("a"));
    assert_eq!(state.postman.workspaces.len(), 2);
}

#[test]
fn the_key_is_kept_or_forgotten_only_after_postman_accepts_it() {
    let mut state = state_with(Workspace::default());
    state.postman.api_key = KEY.to_string();
    state.postman.remember_key = false;
    let effects = state.apply(Action::ListPostmanWorkspaces, 0.0);
    let [Effect::ListPostmanWorkspaces { job, key, .. }] = effects.as_slice() else {
        panic!("only the listing starts, got {effects:?}");
    };
    assert_eq!(key.expose(), KEY);

    let effects = state.apply(
        Action::PostmanWorkspacesListed {
            job: *job,
            listed: Ok(vec![workspace("a")]),
        },
        0.0,
    );
    assert!(matches!(
        effects.as_slice(),
        [Effect::StorePostmanKey(None)]
    ));
}

#[test]
fn a_late_listing_is_ignored() {
    let mut state = listed_state();
    let effects = state.apply(
        Action::PostmanWorkspacesListed {
            job: 99,
            listed: Ok(Vec::new()),
        },
        0.0,
    );
    assert!(effects.is_empty());
    assert_eq!(state.postman.workspaces.len(), 2);
}

#[test]
fn editing_the_key_forgets_the_listed_workspaces() {
    let mut state = listed_state();
    state.postman.key_edited();
    assert!(state.postman.workspaces.is_empty());
    assert!(!state.postman.can_migrate());
    assert!(state.apply(Action::MigratePostmanWorkspace, 0.0).is_empty());
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

    let late = MigrationProgress {
        done: 1,
        total: 2,
        current: "Staging".to_string(),
    };
    state.apply(
        Action::MigrationProgressed {
            job,
            progress: late,
        },
        0.0,
    );
    assert!(state.postman.progress.is_none());
}

#[test]
fn the_form_never_prints_the_key() {
    let state = listed_state();
    assert!(!format!("{:?}", state.postman).contains(KEY));
}
