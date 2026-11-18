use std::collections::BTreeMap;

use super::{MemoryVaultStore, SecretVault, VaultStore};
use crate::collection::{Collection, Variable};
use crate::environment::{Environment, SECRET_KIND};
use crate::secret::Secret;
use crate::settings::ProxySettings;
use crate::workspace::Workspace;

const TOKEN: &str = "token";
const TOKEN_VALUE: &str = "s3cr3t";
const HOST: &str = "host";
const HOST_VALUE: &str = "example.com";
const PROXY_PASSWORD: &str = "proxy-pass";

fn environment_with_secret() -> Environment {
    let mut environment = Environment::new("Staging");
    environment.set_value(HOST, HOST_VALUE);
    environment.set_value(TOKEN, TOKEN_VALUE);
    environment.values[1].set_secret(true);
    environment
}

fn workspace_with_secrets() -> Workspace {
    let mut workspace = Workspace::default();
    workspace.add_environment(environment_with_secret());
    let mut proxy = ProxySettings {
        url: "http://proxy.local:8080".to_string(),
        ..ProxySettings::default()
    };
    proxy.set_password_text(PROXY_PASSWORD);
    workspace.settings.request.proxy = Some(proxy);
    workspace
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
fn only_filled_secret_values_and_the_proxy_password_are_collected() {
    let workspace = workspace_with_secrets();
    let vault = SecretVault::collect(&workspace);
    let id = workspace.environments[0].id.clone().unwrap();
    let expected = BTreeMap::from([(TOKEN.to_string(), Secret::new(TOKEN_VALUE))]);
    assert_eq!(vault.environments.get(&id), Some(&expected));
    assert_eq!(vault.proxy_password, Some(Secret::new(PROXY_PASSWORD)));
    assert!(SecretVault::collect(&stripped(&workspace)).is_empty());
}

#[test]
fn restoring_fills_the_stripped_values_back_in() {
    let workspace = workspace_with_secrets();
    let vault = SecretVault::collect(&workspace);
    let mut restored = stripped(&workspace);
    vault.restore_into(&mut restored);
    assert_eq!(restored, workspace);
}

#[test]
fn restoring_never_overwrites_a_value_on_disk() {
    let workspace = workspace_with_secrets();
    let vault = SecretVault::collect(&workspace);
    let mut edited = workspace.clone();
    edited.environments[0].set_value(TOKEN, "newer");
    vault.restore_into(&mut edited);
    assert_eq!(
        edited.environments[0].value_of(TOKEN).as_deref(),
        Some("newer")
    );
}

#[test]
fn the_vault_survives_a_json_round_trip() {
    let vault = SecretVault::collect(&workspace_with_secrets());
    let text = vault.to_json().unwrap();
    assert_eq!(SecretVault::from_json(&text).unwrap(), vault);
    assert_eq!(SecretVault::default().to_json().unwrap(), "{}");
    assert!(SecretVault::from_json("not json").is_err());
}

#[test]
fn saving_an_empty_vault_forgets_the_saved_one() {
    let store = MemoryVaultStore::holding(SecretVault::collect(&workspace_with_secrets()));
    store.save(&SecretVault::default()).unwrap();
    assert_eq!(store.load().unwrap(), None);
    assert!(MemoryVaultStore::unavailable().load().is_err());
}

#[test]
fn exports_leave_secret_values_empty() {
    let environment = environment_with_secret();
    let exported = Environment::from_json(&environment.to_export_json().unwrap()).unwrap();
    assert_eq!(exported.value_of(TOKEN).as_deref(), Some(""));
    assert_eq!(exported.value_of(HOST).as_deref(), Some(HOST_VALUE));
    assert!(exported.values[1].is_secret());

    let mut collection = Collection::new("Shared");
    let mut variable = Variable::text(TOKEN, TOKEN_VALUE);
    variable.kind = Some(SECRET_KIND.to_string());
    collection.variable = Some(vec![variable]);
    let exported = Collection::from_json(&collection.to_export_json().unwrap()).unwrap();
    let variables = exported.variable.unwrap();
    assert_eq!(variables[0].value_text(), "");
    assert!(variables[0].is_secret());
}
