mod keychain;
mod memory;

pub use keychain::KeychainVaultStore;
pub use memory::MemoryVaultStore;

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::secret::Secret;
use crate::workspace::Workspace;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SecretVault {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    environments: BTreeMap<String, BTreeMap<String, Secret>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    proxy_password: Option<Secret>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultError(pub String);

pub trait VaultStore: Send + Sync + fmt::Debug {
    fn load(&self) -> Result<Option<SecretVault>, VaultError>;
    fn save(&self, vault: &SecretVault) -> Result<(), VaultError>;
}

impl SecretVault {
    pub fn collect(workspace: &Workspace) -> Self {
        SecretVault {
            environments: workspace
                .environments
                .iter()
                .filter_map(|environment| {
                    let values = environment.secret_values();
                    let id = environment.id.clone()?;
                    (!values.is_empty()).then_some((id, values))
                })
                .collect(),
            proxy_password: workspace.settings.proxy_password().cloned(),
        }
    }

    pub fn restore_into(&self, workspace: &mut Workspace) {
        for environment in &mut workspace.environments {
            let saved = environment
                .id
                .as_ref()
                .and_then(|id| self.environments.get(id));
            if let Some(saved) = saved {
                environment.restore_secret_values(saved);
            }
        }
        workspace
            .settings
            .restore_proxy_password(self.proxy_password.as_ref());
    }

    pub fn is_empty(&self) -> bool {
        self.environments.is_empty() && self.proxy_password.is_none()
    }

    pub fn from_json(text: &str) -> Result<Self, VaultError> {
        serde_json::from_str(text).map_err(|error| VaultError(error.to_string()))
    }

    pub fn to_json(&self) -> Result<String, VaultError> {
        serde_json::to_string(self).map_err(|error| VaultError(error.to_string()))
    }
}

impl fmt::Display for VaultError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for VaultError {}

#[cfg(test)]
mod tests;
