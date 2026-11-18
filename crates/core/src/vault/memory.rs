use std::collections::BTreeMap;
use std::sync::Mutex;

use super::{VaultError, VaultSlot, VaultStore};
use crate::secret::Secret;

const UNAVAILABLE: &str = "no keychain is available";
const REFUSED: &str = "the keychain refused the value";

#[derive(Debug, Default)]
pub struct MemoryVaultStore {
    saved: Mutex<BTreeMap<VaultSlot, Secret>>,
    unavailable: bool,
    refusing_saves: bool,
}

impl MemoryVaultStore {
    pub fn unavailable() -> Self {
        MemoryVaultStore {
            unavailable: true,
            ..MemoryVaultStore::default()
        }
    }

    pub fn refusing_saves() -> Self {
        MemoryVaultStore {
            refusing_saves: true,
            ..MemoryVaultStore::default()
        }
    }

    pub fn holding(slot: VaultSlot, value: Secret) -> Self {
        MemoryVaultStore {
            saved: Mutex::new(BTreeMap::from([(slot, value)])),
            ..MemoryVaultStore::default()
        }
    }

    pub fn saved(&self, slot: &VaultSlot) -> Option<Secret> {
        self.saved.lock().ok()?.get(slot).cloned()
    }

    fn slots(&self) -> Result<std::sync::MutexGuard<'_, BTreeMap<VaultSlot, Secret>>, VaultError> {
        if self.unavailable {
            return Err(VaultError::new(UNAVAILABLE));
        }
        self.saved.lock().map_err(VaultError::new)
    }
}

impl VaultStore for MemoryVaultStore {
    fn load(&self, slot: &VaultSlot) -> Result<Option<Secret>, VaultError> {
        Ok(self.slots()?.get(slot).cloned())
    }

    fn save(&self, slot: &VaultSlot, value: Option<&Secret>) -> Result<(), VaultError> {
        if self.refusing_saves {
            return Err(VaultError::new(REFUSED));
        }
        let mut slots = self.slots()?;
        match value {
            Some(value) => slots.insert(slot.clone(), value.clone()),
            None => slots.remove(slot),
        };
        Ok(())
    }
}
