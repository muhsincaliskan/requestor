use std::collections::BTreeMap;

use super::{EnvironmentSecrets, MemoryVaultStore, SecretOrigin, VaultSlot, VaultStore};
use crate::collection::{Collection, Item, Variable};
use crate::environment::{Environment, EnvironmentValue, SECRET_KIND};
use crate::secret::Secret;
use crate::settings::ProxySettings;
use crate::workspace::Workspace;

const TOKEN: &str = "token";
const TOKEN_VALUE: &str = "s3cr3t";
const OTHER_TOKEN_VALUE: &str = "other";
const HOST: &str = "host";
const HOST_VALUE: &str = "example.com";
const PROXY_URL: &str = "http://proxy.local:8080";
const PROXY_PASSWORD: &str = "proxy-pass";

fn secret_value(environment: &mut Environment, key: &str, value: &str) {
    environment.values.push(EnvironmentValue::blank());
    let entry = environment.values.last_mut().unwrap();
    entry.key = key.to_string();
    entry.set_value_text(value);
    entry.set_secret(true);
}

fn environment_with_secret() -> Environment {
    let mut environment = Environment::new("Staging");
    environment.set_value(HOST, HOST_VALUE);
    secret_value(&mut environment, TOKEN, TOKEN_VALUE);
    environment
}

fn workspace_with_secrets() -> Workspace {
    let mut workspace = Workspace::default();
    workspace.add_environment(environment_with_secret());
    let mut proxy = ProxySettings {
        url: PROXY_URL.to_string(),
        ..ProxySettings::default()
    };
    proxy.set_password_text(PROXY_PASSWORD);
    workspace.settings.request.proxy = Some(proxy);
    workspace
}

fn vault_of(workspace: &Workspace) -> BTreeMap<VaultSlot, Secret> {
    let environment = &workspace.environments[0];
    BTreeMap::from([
        (
            environment.vault_slot(),
            environment.secrets().to_secret().unwrap(),
        ),
        (VaultSlot::proxy_password(), Secret::new(PROXY_PASSWORD)),
    ])
}

fn stripped(workspace: &Workspace) -> Workspace {
    let mut stripped = workspace.clone();
    stripped.environments = workspace
        .environments
        .iter()
        .map(Environment::without_secret_values)
        .collect();
    stripped.settings = workspace.settings.without_secrets();
    stripped
}

#[test]
fn only_secret_environments_and_a_proxy_need_slots() {
    let mut workspace = workspace_with_secrets();
    workspace.add_environment(Environment::new("Plain"));
    let slots = workspace.vault_slots();
    assert_eq!(
        slots,
        vec![
            workspace.environments[0].vault_slot(),
            VaultSlot::proxy_password()
        ]
    );
}

#[test]
fn restoring_fills_the_stripped_values_back_in() {
    let workspace = workspace_with_secrets();
    let mut restored = stripped(&workspace);
    let restore = restored.restore_secrets(&vault_of(&workspace));
    assert_eq!(restored, workspace);
    assert!(restore.unsecured.environments.is_empty());
    assert!(!restore.unsecured.settings);
    assert!(restore.missing.is_empty());
}

#[test]
fn secrets_still_in_files_are_flagged_and_never_overwritten() {
    let workspace = workspace_with_secrets();
    let mut edited = workspace.clone();
    edited.environments[0].set_value(TOKEN, "newer");
    let restore = edited.restore_secrets(&vault_of(&workspace));
    assert_eq!(
        edited.environments[0].value_of(TOKEN).as_deref(),
        Some("newer")
    );
    assert_eq!(restore.unsecured.environments.len(), 1);
    assert!(restore.unsecured.settings);
}

#[test]
fn blank_secrets_without_a_saved_entry_are_reported_missing() {
    let workspace = workspace_with_secrets();
    let mut blank = stripped(&workspace);
    let restore = blank.restore_secrets(&BTreeMap::new());
    assert_eq!(restore.missing, vec!["Staging".to_string()]);
    assert_eq!(restore.unsecured.environments.len(), 1);
    let unreadable = BTreeMap::from([(blank.environments[0].vault_slot(), Secret::new("{"))]);
    let restore = stripped(&workspace).restore_secrets(&unreadable);
    assert_eq!(restore.unreadable, vec!["Staging".to_string()]);
}

#[test]
fn duplicate_secret_keys_keep_their_own_values() {
    let mut environment = environment_with_secret();
    secret_value(&mut environment, TOKEN, OTHER_TOKEN_VALUE);
    let saved = environment.secrets();
    let mut restored = environment.without_secret_values();
    restored.restore_secrets(&saved);
    assert_eq!(restored, environment);
    let secret = saved.to_secret().unwrap();
    assert_eq!(EnvironmentSecrets::from_secret(&secret).unwrap(), saved);
    let failure = EnvironmentSecrets::from_secret(&Secret::new("[1234]")).unwrap_err();
    assert!(!failure.to_string().contains("1234"), "{failure}");
}

#[test]
fn an_environment_reports_where_its_secrets_came_from() {
    let environment = environment_with_secret();
    let saved = environment.secrets().to_secret().unwrap();
    let mut blank = environment.without_secret_values();
    assert_eq!(blank.restore_from_vault(None), SecretOrigin::Missing);
    assert_eq!(blank.restore_from_vault(Some(&saved)), SecretOrigin::Vault);
    assert_eq!(blank, environment);
    assert_eq!(blank.restore_from_vault(Some(&saved)), SecretOrigin::Files);
}

#[test]
fn the_memory_store_saves_and_forgets_slots() {
    let slot = VaultSlot::environment("one");
    let store = MemoryVaultStore::holding(slot.clone(), Secret::new(TOKEN_VALUE));
    assert_eq!(store.load(&slot).unwrap(), Some(Secret::new(TOKEN_VALUE)));
    store.save(&slot, None).unwrap();
    assert_eq!(store.load(&slot).unwrap(), None);
    assert!(MemoryVaultStore::unavailable().load(&slot).is_err());
}

#[test]
fn exports_leave_secret_values_empty() {
    let environment = environment_with_secret();
    let exported = Environment::from_json(&environment.to_export_json().unwrap()).unwrap();
    assert_eq!(exported.values[1].value_text(), "");
    assert_eq!(exported.value_of(HOST).as_deref(), Some(HOST_VALUE));
    assert!(exported.values[1].is_secret());

    let mut variable = Variable::text(TOKEN, TOKEN_VALUE);
    variable.kind = Some(SECRET_KIND.to_string());
    let mut folder = Item::named("Folder".to_string());
    folder.variable = Some(vec![variable.clone()]);
    folder.item = Some(Vec::new());
    let mut collection = Collection::new("Shared");
    collection.variable = Some(vec![variable]);
    collection.item.push(folder);
    let text = collection.to_export_json().unwrap();
    assert!(!text.contains(TOKEN_VALUE), "{text}");
    let exported = Collection::from_json(&text).unwrap();
    assert!(exported.variable.unwrap()[0].is_secret());
}
