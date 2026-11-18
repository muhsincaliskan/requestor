use std::collections::{BTreeMap, BTreeSet};

use super::Workspace;
use crate::environment::Environment;
use crate::secret::Secret;
use crate::vault::{SecretOrigin, VaultSlot};

const MISSING: &str = "no saved secret values for";
const UNREADABLE_FOR: &str = "unreadable secret values for";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnsecuredSecrets {
    pub environments: BTreeSet<String>,
    pub settings: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SecretRestore {
    pub unsecured: UnsecuredSecrets,
    pub missing: Vec<String>,
    pub unreadable: Vec<String>,
}

impl SecretRestore {
    pub fn warnings(&self) -> impl Iterator<Item = String> + '_ {
        let missing = self.missing.iter().map(|name| format!("{MISSING} {name}"));
        let unreadable = self
            .unreadable
            .iter()
            .map(|name| format!("{UNREADABLE_FOR} {name}"));
        missing.chain(unreadable)
    }

    fn record(&mut self, environment: &Environment, origin: SecretOrigin) {
        let id = environment.id.clone().unwrap_or_default();
        match origin {
            SecretOrigin::Vault => {}
            SecretOrigin::Files => {
                self.unsecured.environments.insert(id);
            }
            SecretOrigin::Missing => {
                self.unsecured.environments.insert(id);
                self.missing.push(environment.name.clone());
            }
            SecretOrigin::Unreadable => self.unreadable.push(environment.name.clone()),
        }
    }
}

impl Workspace {
    pub fn vault_slots(&self) -> Vec<VaultSlot> {
        let environments = self
            .environments
            .iter()
            .filter(|environment| environment.declares_secrets())
            .map(Environment::vault_slot);
        let proxy = self.settings.uses_proxy().then(VaultSlot::proxy_password);
        environments.chain(proxy).collect()
    }

    pub fn restore_secrets(&mut self, saved: &BTreeMap<VaultSlot, Secret>) -> SecretRestore {
        let mut restore = SecretRestore::default();
        let declaring = self
            .environments
            .iter_mut()
            .filter(|environment| environment.declares_secrets());
        for environment in declaring {
            let origin = environment.restore_from_vault(saved.get(&environment.vault_slot()));
            restore.record(environment, origin);
        }
        let proxy = saved.get(&VaultSlot::proxy_password());
        restore.unsecured.settings = self.settings.restore_from_vault(proxy) == SecretOrigin::Files;
        restore
    }
}
