use std::sync::Mutex;

use super::{SecretVault, VaultError, VaultStore};

const UNAVAILABLE: &str = "no keychain is available";

#[derive(Debug, Default)]
pub struct MemoryVaultStore {
    saved: Mutex<Option<SecretVault>>,
    unavailable: bool,
}

impl MemoryVaultStore {
    pub fn unavailable() -> Self {
        MemoryVaultStore {
            saved: Mutex::default(),
            unavailable: true,
        }
    }

    pub fn holding(vault: SecretVault) -> Self {
        MemoryVaultStore {
            saved: Mutex::new(Some(vault)),
            unavailable: false,
        }
    }

    pub fn saved(&self) -> Option<SecretVault> {
        self.saved.lock().ok()?.clone()
    }

    fn check(&self) -> Result<(), VaultError> {
        if self.unavailable {
            Err(VaultError(UNAVAILABLE.to_string()))
        } else {
            Ok(())
        }
    }
}

impl VaultStore for MemoryVaultStore {
    fn load(&self) -> Result<Option<SecretVault>, VaultError> {
        self.check()?;
        Ok(self.saved())
    }

    fn save(&self, vault: &SecretVault) -> Result<(), VaultError> {
        self.check()?;
        let mut saved = self
            .saved
            .lock()
            .map_err(|error| VaultError(error.to_string()))?;
        *saved = (!vault.is_empty()).then(|| vault.clone());
        Ok(())
    }
}
