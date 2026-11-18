use std::fmt;
use std::path::Path;

use super::pending_write::StoreError;
use super::vault_write::KEYCHAIN_LABEL;
use super::{PendingWrite, VaultWrite};

const KEPT_IN_FILE: &str = "the secret values stay in";

#[derive(Clone, PartialEq)]
pub struct SecuredWrite {
    vault: VaultWrite,
    stripped: PendingWrite,
    whole: PendingWrite,
}

impl SecuredWrite {
    pub(super) fn new(vault: VaultWrite, stripped: PendingWrite, whole: PendingWrite) -> Self {
        SecuredWrite {
            vault,
            stripped,
            whole,
        }
    }

    pub fn path(&self) -> &Path {
        self.whole.path()
    }

    pub fn commit(self) -> Result<(), StoreError> {
        let Err(error) = self.vault.save() else {
            return self.stripped.commit();
        };
        let path = self.whole.path().to_path_buf();
        self.whole.commit()?;
        Err(StoreError::at(
            Path::new(KEYCHAIN_LABEL),
            format!("{error}; {KEPT_IN_FILE} {}", path.display()),
        ))
    }
}

impl fmt::Debug for SecuredWrite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecuredWrite")
            .field("vault", &self.vault)
            .field("path", &self.path())
            .finish_non_exhaustive()
    }
}
