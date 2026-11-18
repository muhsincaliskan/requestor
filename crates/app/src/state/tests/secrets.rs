use std::sync::Arc;

use requestor_core::environment::Environment;
use requestor_core::identity::random_uuid;
use requestor_core::vault::MemoryVaultStore;
use requestor_core::workspace::{LoadedWorkspace, StoreWrite, Workspace, WorkspaceStore};

use super::LATER;
use crate::state::{Action, AppState, Effect, LibraryTarget};

const TOKEN: &str = "token";
const TOKEN_VALUE: &str = "s3cr3t";

fn workspace_with_secret() -> (Workspace, String) {
    let mut workspace = Workspace::default();
    let mut environment = Environment::new("Staging");
    environment.set_value(TOKEN, TOKEN_VALUE);
    environment.values[0].set_secret(true);
    let id = workspace.add_environment(environment);
    (workspace, id)
}

fn secured_state(workspace: Workspace, secrets_to_secure: bool) -> AppState {
    let root = std::env::temp_dir().join(format!("requestor-secrets-{}", random_uuid()));
    let mut store = WorkspaceStore::new(root).with_vault(Arc::new(MemoryVaultStore::default()));
    store.load();
    let loaded = LoadedWorkspace {
        workspace,
        secrets_to_secure,
        ..LoadedWorkspace::default()
    };
    AppState::new(loaded, store)
}

fn single_batch(effects: Vec<Effect>) -> Vec<StoreWrite> {
    let mut batches: Vec<Vec<StoreWrite>> = effects
        .into_iter()
        .filter_map(|effect| match effect {
            Effect::Persist(StoreWrite::Batch(writes)) => Some(writes),
            _ => None,
        })
        .collect();
    assert_eq!(batches.len(), 1, "expected one batch");
    batches.remove(0)
}

#[test]
fn secrets_found_in_files_are_rewritten_through_the_keychain_on_the_first_save() {
    let (workspace, _) = workspace_with_secret();
    let mut state = secured_state(workspace, true);
    let writes = single_batch(state.tick(LATER));
    assert!(matches!(
        writes.as_slice(),
        [StoreWrite::Vault(_), StoreWrite::File(_)]
    ));
}

#[test]
fn an_edited_environment_saves_the_keychain_before_its_file() {
    let (workspace, id) = workspace_with_secret();
    let mut state = secured_state(workspace, false);
    assert!(state.tick(LATER).is_empty());
    state.unsaved.touch_environment(&id, LATER);
    let writes = single_batch(state.tick(LATER * 2.0));
    assert!(matches!(
        writes.as_slice(),
        [StoreWrite::Vault(_), StoreWrite::File(_)]
    ));
}

#[test]
fn deleting_an_environment_forgets_its_secrets() {
    let (workspace, environment_id) = workspace_with_secret();
    let mut state = secured_state(workspace, false);
    let target = LibraryTarget::Environment { environment_id };
    state.apply(Action::Delete(target), 0.0);
    let writes = single_batch(state.tick(LATER));
    assert!(matches!(writes.as_slice(), [StoreWrite::Vault(_)]));
}

#[test]
fn exported_environments_leave_secret_values_empty() {
    let (workspace, environment_id) = workspace_with_secret();
    let mut state = super::state_with(workspace);
    let target = LibraryTarget::Environment { environment_id };
    let file_path = "Staging.postman_environment.json".into();
    let effects = state.apply(Action::Export { target, file_path }, 0.0);
    let [Effect::WriteFile { contents, .. }] = effects.as_slice() else {
        panic!("expected one written file, got {effects:?}");
    };
    assert!(!contents.contains(TOKEN_VALUE), "{contents}");
}
