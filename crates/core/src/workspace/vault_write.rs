use std::fmt;
use std::path::Path;
use std::sync::Arc;

use super::pending_write::StoreError;
use crate::secret::Secret;
use crate::vault::{VaultError, VaultSlot, VaultStore};

pub(super) const KEYCHAIN_LABEL: &str = "the keychain";

#[derive(Clone)]
pub struct VaultWrite {
    store: Arc<dyn VaultStore>,
    slot: VaultSlot,
    value: Option<Secret>,
}

impl VaultWrite {
    pub(super) fn new(store: Arc<dyn VaultStore>, slot: VaultSlot, value: Option<Secret>) -> Self {
        VaultWrite { store, slot, value }
    }

    pub fn path(&self) -> &Path {
        Path::new(KEYCHAIN_LABEL)
    }

    pub fn commit(self) -> Result<(), StoreError> {
        self.save()
            .map_err(|error| StoreError::at(Path::new(KEYCHAIN_LABEL), error))
    }

    pub(super) fn save(&self) -> Result<(), VaultError> {
        self.store.save(&self.slot, self.value.as_ref())
    }
}

impl fmt::Debug for VaultWrite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VaultWrite")
            .field("slot", &self.slot)
            .finish_non_exhaustive()
    }
}

impl PartialEq for VaultWrite {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot && self.value == other.value
    }
}
