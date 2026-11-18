use std::path::Path;

use keyring::{Entry, Error};

use super::{SecretVault, VaultError, VaultStore};

const SERVICE: &str = "Requestor";
const ACCOUNT_PREFIX: &str = "secrets:";

#[derive(Debug)]
pub struct KeychainVaultStore {
    account: String,
}

impl KeychainVaultStore {
    pub fn for_workspace(root: &Path) -> Self {
        KeychainVaultStore {
            account: format!("{ACCOUNT_PREFIX}{}", root.display()),
        }
    }

    fn entry(&self) -> Result<Entry, VaultError> {
        Entry::new(SERVICE, &self.account).map_err(vault_error)
    }
}

impl VaultStore for KeychainVaultStore {
    fn load(&self) -> Result<Option<SecretVault>, VaultError> {
        match self.entry()?.get_password() {
            Ok(text) => SecretVault::from_json(&text).map(Some),
            Err(Error::NoEntry) => Ok(None),
            Err(error) => Err(vault_error(error)),
        }
    }

    fn save(&self, vault: &SecretVault) -> Result<(), VaultError> {
        let entry = self.entry()?;
        if vault.is_empty() {
            return match entry.delete_credential() {
                Ok(()) | Err(Error::NoEntry) => Ok(()),
                Err(error) => Err(vault_error(error)),
            };
        }
        entry.set_password(&vault.to_json()?).map_err(vault_error)
    }
}

fn vault_error(error: Error) -> VaultError {
    VaultError(error.to_string())
}
