use std::path::Path;
use std::sync::Arc;

use super::super::pending_write::StoreError;
use super::super::vault_write::{VaultWrite, KEYCHAIN_LABEL};
use super::super::{StoreWrite, Workspace};
use super::WorkspaceStore;
use crate::environment::Environment;
use crate::vault::{SecretVault, VaultStore};

const KEPT_IN_FILES: &str = "secrets stay in the workspace files";

impl WorkspaceStore {
    pub fn with_vault(mut self, vault: Arc<dyn VaultStore>) -> Self {
        self.vault = Some(vault);
        self
    }

    pub fn keeps_secrets_in_vault(&self) -> bool {
        self.secured
    }

    pub fn vault_write(&self, workspace: &Workspace) -> Option<StoreWrite> {
        let vault = self.vault.as_ref().filter(|_| self.secured)?;
        let secrets = SecretVault::collect(workspace);
        Some(StoreWrite::Vault(VaultWrite::new(
            Arc::clone(vault),
            secrets,
        )))
    }

    pub(super) fn open_vault(
        &mut self,
        workspace: &mut Workspace,
        warnings: &mut Vec<StoreError>,
    ) -> bool {
        let on_disk = !SecretVault::collect(workspace).is_empty();
        let declared = on_disk
            || workspace
                .environments
                .iter()
                .any(Environment::declares_secrets);
        let Some(vault) = &self.vault else {
            return false;
        };
        match vault.load() {
            Ok(saved) => {
                if let Some(saved) = saved {
                    saved.restore_into(workspace);
                }
                self.secured = true;
                on_disk
            }
            Err(error) => {
                if declared {
                    let reason = format!("{KEPT_IN_FILES} ({error})");
                    warnings.push(StoreError::at(Path::new(KEYCHAIN_LABEL), reason));
                }
                false
            }
        }
    }
}
