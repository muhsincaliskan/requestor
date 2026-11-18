use std::collections::BTreeSet;
use std::sync::Arc;

use requestor_core::environment::Environment;
use requestor_core::identity::random_uuid;
use requestor_core::vault::MemoryVaultStore;
use requestor_core::workspace::{
    LoadedWorkspace, StoreWrite, UnsecuredSecrets, Workspace, WorkspaceStore,
};

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

fn secured_state(workspace: Workspace, unsecured: UnsecuredSecrets) -> AppState {
    let root = std::env::temp_dir().join(format!("requestor-secrets-{}", random_uuid()));
    let mut store = WorkspaceStore::new(root).with_vault(Arc::new(MemoryVaultStore::default()));
    store.load();
    let loaded = LoadedWorkspace {
        workspace,
        unsecured,
        ..LoadedWorkspace::default()
    };
    AppState::new(loaded, store)
}

fn secured_writes(effects: &[Effect]) -> usize {
    effects
        .iter()
        .filter(|effect| matches!(effect, Effect::Persist(StoreWrite::Secured(_))))
        .count()
}

#[test]
fn secrets_found_in_files_are_rewritten_through_the_keychain_on_the_first_save() {
    let (workspace, id) = workspace_with_secret();
    let unsecured = UnsecuredSecrets {
        environments: BTreeSet::from([id]),
        settings: true,
    };
    let mut state = secured_state(workspace, unsecured);
    assert_eq!(secured_writes(&state.tick(LATER)), 2);
}

#[test]
fn an_edited_environment_goes_through_the_keychain() {
    let (workspace, id) = workspace_with_secret();
    let mut state = secured_state(workspace, UnsecuredSecrets::default());
    assert!(state.tick(LATER).is_empty());
    state.unsaved.touch_environment(&id, LATER);
    assert_eq!(secured_writes(&state.tick(LATER * 2.0)), 1);
}

#[test]
fn deleting_an_environment_forgets_its_secrets() {
    let (workspace, environment_id) = workspace_with_secret();
    let mut state = secured_state(workspace, UnsecuredSecrets::default());
    let target = LibraryTarget::Environment {
        environment_id: environment_id.clone(),
    };
    let effects = state.apply(Action::Delete(target), 0.0);
    let forgotten = effects.iter().any(|effect| {
        matches!(effect, Effect::Persist(write @ StoreWrite::Vault(_))
            if *write == state.store.forget_environment_secrets(&environment_id).unwrap())
    });
    assert!(forgotten, "{effects:?}");
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

#[test]
fn exports_leave_out_unmarked_credentials_unless_kept() {
    use requestor_core::environment::Environment;

    let mut workspace = requestor_core::workspace::Workspace::default();
    let mut environment = Environment::new("Staging");
    environment.set_value("apiToken", "plain-token");
    let environment_id = workspace.add_environment(environment);
    let mut state = super::state_with(workspace);
    let target = LibraryTarget::Environment { environment_id };
    assert_eq!(state.exposed_credential_count(&target), 1);

    let export = |state: &mut crate::state::AppState| {
        let file_path = "Staging.postman_environment.json".into();
        let effects = state.apply(
            Action::Export {
                target: target.clone(),
                file_path,
            },
            0.0,
        );
        let [Effect::WriteFile { contents, .. }] = effects.as_slice() else {
            panic!("expected one written file, got {effects:?}");
        };
        contents.clone()
    };
    assert!(!export(&mut state).contains("plain-token"));

    let mut form = crate::state::ExportForm::suggested(target.clone(), "Staging");
    form.keep_credentials = true;
    state.overlay = Some(crate::state::Overlay::Export(form));
    assert!(export(&mut state).contains("plain-token"));
}
