use std::sync::Arc;

use super::super::{ENVIRONMENTS_DIRECTORY, SETTINGS_FILE};
use super::TemporaryRoot;
use crate::environment::Environment;
use crate::settings::ProxySettings;
use crate::vault::{MemoryVaultStore, VaultSlot};
use crate::workspace::{StoreWrite, Workspace, WorkspaceStore};

const TOKEN: &str = "token";
const TOKEN_VALUE: &str = "s3cr3t";
const PROXY_URL: &str = "http://proxy.local:8080";
const PROXY_PASSWORD: &str = "proxy-pass";

fn workspace_with_secrets() -> Workspace {
    let mut workspace = Workspace::default();
    let mut environment = Environment::new("Staging");
    environment.set_value(TOKEN, TOKEN_VALUE);
    environment.values[0].set_secret(true);
    workspace.add_environment(environment);
    let mut proxy = ProxySettings {
        url: PROXY_URL.to_string(),
        ..ProxySettings::default()
    };
    proxy.set_password_text(PROXY_PASSWORD);
    workspace.settings.request.proxy = Some(proxy);
    workspace
}

fn save(store: &mut WorkspaceStore, workspace: &Workspace) -> Vec<StoreWrite> {
    let mut writes: Vec<StoreWrite> = workspace
        .environments
        .iter()
        .map(|environment| store.environment_write(environment).unwrap())
        .collect();
    writes.push(store.settings_write(&workspace.settings).unwrap());
    writes
}

fn commit_all(writes: Vec<StoreWrite>) -> usize {
    writes
        .into_iter()
        .filter(|write| write.clone().commit().is_err())
        .count()
}

fn files_text(store: &WorkspaceStore) -> String {
    let root = store.root();
    let mut text = std::fs::read_to_string(root.join(SETTINGS_FILE)).unwrap();
    for entry in std::fs::read_dir(root.join(ENVIRONMENTS_DIRECTORY)).unwrap() {
        text.push_str(&std::fs::read_to_string(entry.unwrap().path()).unwrap());
    }
    text
}

fn with_vault(root: &TemporaryRoot, vault: &Arc<MemoryVaultStore>) -> WorkspaceStore {
    root.store().with_vault(Arc::clone(vault) as _)
}

#[test]
fn a_store_without_a_keychain_keeps_secrets_in_its_files() {
    let root = TemporaryRoot::new();
    let mut store = root.store();
    let workspace = workspace_with_secrets();
    assert_eq!(commit_all(save(&mut store, &workspace)), 0);
    assert!(files_text(&store).contains(TOKEN_VALUE));
    assert_eq!(store.load().workspace, workspace);
}

#[test]
fn secrets_go_to_the_keychain_and_come_back_on_load() {
    let root = TemporaryRoot::new();
    let vault = Arc::new(MemoryVaultStore::default());
    let mut store = with_vault(&root, &vault);
    assert!(store.load().unsecured.environments.is_empty());
    let workspace = workspace_with_secrets();
    assert_eq!(commit_all(save(&mut store, &workspace)), 0);
    let files = files_text(&store);
    assert!(!files.contains(TOKEN_VALUE), "{files}");
    assert!(!files.contains(PROXY_PASSWORD), "{files}");
    assert!(vault.saved(&VaultSlot::proxy_password()).is_some());
    let reloaded = with_vault(&root, &vault).load();
    assert!(reloaded.warnings.is_empty(), "{:?}", reloaded.warnings);
    assert_eq!(reloaded.unsecured, Default::default());
    assert_eq!(reloaded.workspace, workspace);
}

#[test]
fn secrets_found_in_files_are_flagged_for_moving_to_the_keychain() {
    let root = TemporaryRoot::new();
    let workspace = workspace_with_secrets();
    commit_all(save(&mut root.store(), &workspace));
    let vault = Arc::new(MemoryVaultStore::default());
    let mut store = with_vault(&root, &vault);
    let loaded = store.load();
    assert_eq!(loaded.unsecured.environments.len(), 1);
    assert!(loaded.unsecured.settings);
    assert_eq!(loaded.workspace, workspace);
    commit_all(save(&mut store, &loaded.workspace));
    assert!(!files_text(&store).contains(TOKEN_VALUE));
    assert_eq!(with_vault(&root, &vault).load().workspace, workspace);
}

#[test]
fn a_damaged_environment_file_keeps_its_secrets_in_the_keychain() {
    let root = TemporaryRoot::new();
    let vault = Arc::new(MemoryVaultStore::default());
    let mut store = with_vault(&root, &vault);
    store.load();
    let mut workspace = workspace_with_secrets();
    let mut other = workspace.environments[0].clone();
    other.id = None;
    let other_id = workspace.add_environment(other);
    commit_all(save(&mut store, &workspace));
    let damaged = store
        .environment_write(&workspace.environments[1])
        .unwrap()
        .path()
        .to_path_buf();
    std::fs::write(&damaged, "{ not json").unwrap();
    let mut store = with_vault(&root, &vault);
    let loaded = store.load();
    assert_eq!(loaded.workspace.environments.len(), 1);
    commit_all(save(&mut store, &loaded.workspace));
    assert!(vault.saved(&VaultSlot::environment(&other_id)).is_some());
}

#[test]
fn a_keychain_that_refuses_a_save_leaves_the_secret_in_the_file() {
    let root = TemporaryRoot::new();
    let mut store = with_vault(&root, &Arc::new(MemoryVaultStore::refusing_saves()));
    assert!(store.load().warnings.is_empty());
    let workspace = workspace_with_secrets();
    assert_eq!(commit_all(save(&mut store, &workspace)), 2);
    assert!(files_text(&store).contains(TOKEN_VALUE));
    assert_eq!(store.load().unsecured.environments.len(), 1);
}

#[test]
fn an_unavailable_keychain_never_secures_an_empty_workspace() {
    let root = TemporaryRoot::new();
    let mut store = with_vault(&root, &Arc::new(MemoryVaultStore::unavailable()));
    assert!(store.load().warnings.is_empty());
    let workspace = workspace_with_secrets();
    assert_eq!(commit_all(save(&mut store, &workspace)), 0);
    assert!(files_text(&store).contains(TOKEN_VALUE));
}

#[test]
fn deleting_an_environment_forgets_its_keychain_entry() {
    let root = TemporaryRoot::new();
    let vault = Arc::new(MemoryVaultStore::default());
    let mut store = with_vault(&root, &vault);
    store.load();
    let workspace = workspace_with_secrets();
    commit_all(save(&mut store, &workspace));
    let id = workspace.environments[0].id.clone().unwrap();
    assert!(vault.saved(&VaultSlot::environment(&id)).is_some());
    store
        .forget_environment_secrets(&id)
        .unwrap()
        .commit()
        .unwrap();
    assert!(vault.saved(&VaultSlot::environment(&id)).is_none());
    assert!(root.store().forget_environment_secrets(&id).is_none());
}

#[test]
fn an_unavailable_keychain_keeps_files_whole_and_warns_about_secrets() {
    let root = TemporaryRoot::new();
    let workspace = workspace_with_secrets();
    commit_all(save(&mut root.store(), &workspace));
    let mut store = with_vault(&root, &Arc::new(MemoryVaultStore::unavailable()));
    let loaded = store.load();
    assert_eq!(loaded.warnings.len(), 1);
    assert_eq!(commit_all(save(&mut store, &loaded.workspace)), 0);
    assert!(files_text(&store).contains(TOKEN_VALUE));
}
