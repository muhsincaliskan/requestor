use serde::{Deserialize, Serialize};

use super::VaultError;
use crate::secret::Secret;

const UNREADABLE: &str = "the saved secret values could not be read";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretOrigin {
    Vault,
    Files,
    Missing,
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretEntry {
    key: String,
    value: Secret,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EnvironmentSecrets(Vec<SecretEntry>);

impl SecretEntry {
    pub fn new(key: impl Into<String>, value: Secret) -> Self {
        SecretEntry {
            key: key.into(),
            value,
        }
    }
}

impl FromIterator<SecretEntry> for EnvironmentSecrets {
    fn from_iter<T: IntoIterator<Item = SecretEntry>>(entries: T) -> Self {
        EnvironmentSecrets(entries.into_iter().collect())
    }
}

impl EnvironmentSecrets {
    pub fn from_secret(secret: &Secret) -> Result<Self, VaultError> {
        serde_json::from_str(secret.expose()).map_err(|_| VaultError::new(UNREADABLE))
    }

    pub fn to_secret(&self) -> Result<Secret, VaultError> {
        serde_json::to_string(self)
            .map(Secret::new)
            .map_err(VaultError::new)
    }

    pub fn nth(&self, key: &str, position: usize) -> Option<&Secret> {
        self.0
            .iter()
            .filter(|entry| entry.key == key)
            .nth(position)
            .map(|entry| &entry.value)
    }
}
