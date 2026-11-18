mod environment_secrets;
mod memory;
mod slot;

pub use environment_secrets::{EnvironmentSecrets, SecretEntry, SecretOrigin};
pub use memory::MemoryVaultStore;
pub use slot::VaultSlot;

use std::collections::BTreeMap;
use std::fmt;

use crate::secret::Secret;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultError(String);

pub trait VaultStore: Send + Sync + fmt::Debug {
    fn load(&self, slot: &VaultSlot) -> Result<Option<Secret>, VaultError>;
    fn save(&self, slot: &VaultSlot, value: Option<&Secret>) -> Result<(), VaultError>;

    fn load_all(&self, slots: &[VaultSlot]) -> Result<BTreeMap<VaultSlot, Secret>, VaultError> {
        let mut saved = BTreeMap::new();
        for slot in slots {
            if let Some(secret) = self.load(slot)? {
                saved.insert(slot.clone(), secret);
            }
        }
        Ok(saved)
    }
}

impl VaultError {
    pub fn new(reason: impl fmt::Display) -> Self {
        VaultError(reason.to_string())
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
