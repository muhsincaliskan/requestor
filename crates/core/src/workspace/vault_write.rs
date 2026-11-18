use std::fmt;
use std::path::Path;
use std::sync::Arc;

use super::pending_write::StoreError;
use crate::vault::{SecretVault, VaultStore};

pub(super) const KEYCHAIN_LABEL: &str = "the keychain";

#[derive(Clone)]
pub struct VaultWrite {
    store: Arc<dyn VaultStore>,
    vault: SecretVault,
}

impl VaultWrite {
    pub(super) fn new(store: Arc<dyn VaultStore>, vault: SecretVault) -> Self {
        VaultWrite { store, vault }
    }

    pub fn path(&self) -> &Path {
        Path::new(KEYCHAIN_LABEL)
    }

    pub fn commit(self) -> Result<(), StoreError> {
        self.store
            .save(&self.vault)
            .map_err(|error| StoreError::at(self.path(), error))
    }
}

impl fmt::Debug for VaultWrite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("VaultWrite").finish_non_exhaustive()
    }
}

impl PartialEq for VaultWrite {
    fn eq(&self, other: &Self) -> bool {
        self.vault == other.vault
    }
}
