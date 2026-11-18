use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use keyring::{Entry, Error};
use requestor_core::secret::Secret;
use requestor_core::vault::{VaultError, VaultSlot, VaultStore};

const SERVICE: &str = "Requestor";
const SLOT_SEPARATOR: char = '#';

#[derive(Debug)]
pub struct KeychainVaultStore {
    workspace: PathBuf,
    known: Mutex<HashMap<VaultSlot, Option<Secret>>>,
}

impl KeychainVaultStore {
    pub fn for_workspace(root: &Path) -> Self {
        let workspace = std::fs::canonicalize(root)
            .or_else(|_| std::path::absolute(root))
            .unwrap_or_else(|_| root.to_path_buf());
        KeychainVaultStore {
            workspace,
            known: Mutex::default(),
        }
    }

    fn entry(&self, slot: &VaultSlot) -> Result<Entry, VaultError> {
        let account = format!(
            "{}{SLOT_SEPARATOR}{}",
            self.workspace.display(),
            slot.name()
        );
        Entry::new(SERVICE, &account).map_err(VaultError::new)
    }

    fn is_known(&self, slot: &VaultSlot, value: Option<&Secret>) -> bool {
        self.known
            .lock()
            .is_ok_and(|known| known.get(slot).is_some_and(|saved| saved.as_ref() == value))
    }

    fn remember(&self, slot: &VaultSlot, value: Option<&Secret>) {
        if let Ok(mut known) = self.known.lock() {
            known.insert(slot.clone(), value.cloned());
        }
    }
}

impl VaultStore for KeychainVaultStore {
    fn load(&self, slot: &VaultSlot) -> Result<Option<Secret>, VaultError> {
        let saved = match self.entry(slot)?.get_password() {
            Ok(text) => Some(Secret::new(text)),
            Err(Error::NoEntry) => None,
            Err(error) => return Err(VaultError::new(error)),
        };
        self.remember(slot, saved.as_ref());
        Ok(saved)
    }

    fn save(&self, slot: &VaultSlot, value: Option<&Secret>) -> Result<(), VaultError> {
        if self.is_known(slot, value) {
            return Ok(());
        }
        let entry = self.entry(slot)?;
        let saved = match value {
            Some(value) => entry.set_password(value.expose()),
            None => match entry.delete_credential() {
                Err(Error::NoEntry) => Ok(()),
                other => other,
            },
        };
        saved.map_err(VaultError::new)?;
        self.remember(slot, value);
        Ok(())
    }
}
