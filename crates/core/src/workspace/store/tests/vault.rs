use std::sync::Arc;

use super::TemporaryRoot;
use crate::environment::Environment;
use crate::settings::ProxySettings;
use crate::vault::{MemoryVaultStore, SecretVault};
use crate::workspace::{StoreWrite, Workspace, WorkspaceStore};

const TOKEN: &str = "token";
const TOKEN_VALUE: &str = "s3cr3t";
const PROXY_PASSWORD: &str = "proxy-pass";

fn workspace_with_secrets() -> Workspace {
    let mut workspace = Workspace::default();
    let mut environment = Environment::new("Staging");
    environment.set_value(TOKEN, TOKEN_VALUE);
    environment.values[0].set_secret(true);
    workspace.add_environment(environment);
    let mut proxy = ProxySettings {
        url: "http://proxy.local:8080".to_string(),
        ..ProxySettings::default()
    };
    proxy.set_password_text(PROXY_PASSWORD);
    workspace.settings.request.proxy = Some(proxy);
    workspace
}

fn save(store: &mut WorkspaceStore, workspace: &Workspace) {
    let mut writes: Vec<StoreWrite> = store.vault_write(workspace).into_iter().collect();
    writes.push(
        store
            .environment_write(&workspace.environments[0])
            .unwrap()
            .into(),
    );
    writes.push(store.settings_write(&workspace.settings).unwrap().into());
    StoreWrite::Batch(writes).commit().unwrap();
}

fn files_text(root: &TemporaryRoot) -> String {
    let environments = root.0.join("environments");
    let mut text = std::fs::read_to_string(root.0.join("settings.json")).unwrap();
    for entry in std::fs::read_dir(environments).unwrap() {
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
    assert!(store.vault_write(&workspace).is_none());
    save(&mut store, &workspace);
    assert!(files_text(&root).contains(TOKEN_VALUE));
    assert_eq!(store.load().workspace, workspace);
}

#[test]
fn secrets_go_to_the_keychain_and_come_back_on_load() {
    let root = TemporaryRoot::new();
    let vault = Arc::new(MemoryVaultStore::default());
    let mut store = with_vault(&root, &vault);
    let first = store.load();
    assert!(store.keeps_secrets_in_vault());
    assert!(!first.secrets_to_secure);
    let workspace = workspace_with_secrets();
    save(&mut store, &workspace);
    let files = files_text(&root);
    assert!(!files.contains(TOKEN_VALUE), "{files}");
    assert!(!files.contains(PROXY_PASSWORD), "{files}");
    assert_eq!(vault.saved(), Some(SecretVault::collect(&workspace)));
    let reloaded = with_vault(&root, &vault).load();
    assert!(reloaded.warnings.is_empty());
    assert!(!reloaded.secrets_to_secure);
    assert_eq!(reloaded.workspace, workspace);
}

#[test]
fn secrets_found_in_files_are_flagged_for_moving_to_the_keychain() {
    let root = TemporaryRoot::new();
    let workspace = workspace_with_secrets();
    save(&mut root.store(), &workspace);
    let vault = Arc::new(MemoryVaultStore::default());
    let mut store = with_vault(&root, &vault);
    let loaded = store.load();
    assert!(loaded.secrets_to_secure);
    assert_eq!(loaded.workspace, workspace);
    save(&mut store, &loaded.workspace);
    assert!(!files_text(&root).contains(TOKEN_VALUE));
    assert_eq!(with_vault(&root, &vault).load().workspace, workspace);
}

#[test]
fn an_unavailable_keychain_keeps_files_whole_and_warns_about_secrets() {
    let root = TemporaryRoot::new();
    let workspace = workspace_with_secrets();
    save(&mut root.store(), &workspace);
    let vault = Arc::new(MemoryVaultStore::unavailable());
    let mut store = with_vault(&root, &vault);
    let loaded = store.load();
    assert!(!store.keeps_secrets_in_vault());
    assert!(!loaded.secrets_to_secure);
    assert_eq!(loaded.warnings.len(), 1);
    assert!(store.vault_write(&loaded.workspace).is_none());
    save(&mut store, &loaded.workspace);
    assert!(files_text(&root).contains(TOKEN_VALUE));
}

#[test]
fn an_unavailable_keychain_is_quiet_when_nothing_is_secret() {
    let root = TemporaryRoot::new();
    let vault = Arc::new(MemoryVaultStore::unavailable());
    let loaded = with_vault(&root, &vault).load();
    assert!(loaded.warnings.is_empty());
}
