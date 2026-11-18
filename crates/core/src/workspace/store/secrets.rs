use std::sync::Arc;

use super::super::pending_write::{PendingWrite, StoreError};
use super::super::vault_write::VaultWrite;
use super::super::{SecuredWrite, StoreWrite, UnsecuredSecrets, Workspace};
use super::{pending, WorkspaceStore};
use crate::environment::Environment;
use crate::vault::{VaultSlot, VaultStore};
use crate::workspace::AppSettings;

const KEPT_IN_FILES: &str = "secrets stay in the workspace files";

impl WorkspaceStore {
    pub fn with_vault(mut self, vault: Arc<dyn VaultStore>) -> Self {
        self.vault = Some(vault);
        self
    }

    pub fn forget_environment_secrets(&self, id: &str) -> Option<StoreWrite> {
        let write = VaultWrite::new(self.secured_vault()?, VaultSlot::environment(id), None);
        Some(StoreWrite::Vault(write))
    }

    pub(super) fn secure_environment(
        &self,
        whole: PendingWrite,
        environment: &Environment,
    ) -> Result<StoreWrite, StoreError> {
        let Some(vault) = self.secured_vault() else {
            return Ok(StoreWrite::from(whole));
        };
        let value = if environment.declares_secrets() {
            Some(
                environment
                    .secrets()
                    .to_secret()
                    .map_err(StoreError::keychain)?,
            )
        } else {
            None
        };
        let stripped = pending(
            whole.path().to_path_buf(),
            &environment.without_secret_values(),
        )?;
        let vault = VaultWrite::new(vault, environment.vault_slot(), value);
        Ok(StoreWrite::Secured(SecuredWrite::new(
            vault, stripped, whole,
        )))
    }

    pub(super) fn secure_settings(
        &self,
        whole: PendingWrite,
        settings: &AppSettings,
    ) -> Result<StoreWrite, StoreError> {
        let Some(vault) = self.secured_vault() else {
            return Ok(StoreWrite::from(whole));
        };
        let stripped = pending(whole.path().to_path_buf(), &settings.without_secrets())?;
        let password = settings.proxy_password().cloned();
        let vault = VaultWrite::new(vault, VaultSlot::proxy_password(), password);
        Ok(StoreWrite::Secured(SecuredWrite::new(
            vault, stripped, whole,
        )))
    }

    pub(super) fn open_vault(
        &mut self,
        workspace: &mut Workspace,
        warnings: &mut Vec<StoreError>,
    ) -> UnsecuredSecrets {
        let Some(vault) = self.vault.clone() else {
            return UnsecuredSecrets::default();
        };
        let slots = workspace.vault_slots();
        match vault.load_all(&slots) {
            Ok(saved) => {
                self.secured = true;
                let restore = workspace.restore_secrets(&saved);
                warnings.extend(restore.warnings().map(StoreError::keychain));
                restore.unsecured
            }
            Err(error) => {
                if !slots.is_empty() {
                    let reason = format!("{KEPT_IN_FILES} ({error})");
                    warnings.push(StoreError::keychain(reason));
                }
                UnsecuredSecrets::default()
            }
        }
    }

    fn secured_vault(&self) -> Option<Arc<dyn VaultStore>> {
        self.vault.clone().filter(|_| self.secured)
    }
}
