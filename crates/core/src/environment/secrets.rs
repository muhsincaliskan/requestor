use std::collections::HashMap;

use serde_json::Value;

use super::{Environment, EnvironmentValue};
use crate::error::CoreError;
use crate::secret::Secret;
use crate::vault::{EnvironmentSecrets, SecretEntry, SecretOrigin, VaultSlot};

impl EnvironmentValue {
    pub fn holds_secret(&self) -> bool {
        self.is_secret() && !self.value_text().is_empty()
    }

    fn clear_secret_value(&mut self) {
        if self.is_secret() && self.value.is_some() {
            self.value = Some(Value::String(String::new()));
        }
    }
}

impl Environment {
    pub fn vault_slot(&self) -> VaultSlot {
        VaultSlot::environment(self.id.as_deref().unwrap_or_default())
    }

    pub fn declares_secrets(&self) -> bool {
        self.values.iter().any(EnvironmentValue::is_secret)
    }

    pub fn holds_secrets(&self) -> bool {
        self.values.iter().any(EnvironmentValue::holds_secret)
    }

    pub fn secrets(&self) -> EnvironmentSecrets {
        self.values
            .iter()
            .filter(|value| value.is_secret())
            .map(|value| SecretEntry::new(value.key.clone(), Secret::new(value.value_text())))
            .collect()
    }

    pub fn without_secret_values(&self) -> Environment {
        let mut stripped = self.clone();
        stripped
            .values
            .iter_mut()
            .for_each(EnvironmentValue::clear_secret_value);
        stripped
    }

    pub fn restore_secrets(&mut self, saved: &EnvironmentSecrets) {
        let mut positions: HashMap<String, usize> = HashMap::new();
        for value in self.values.iter_mut().filter(|value| value.is_secret()) {
            let position = positions.entry(value.key.clone()).or_default();
            if value.value_text().is_empty() {
                if let Some(secret) = saved.nth(&value.key, *position) {
                    value.set_value_text(secret.expose());
                }
            }
            *position += 1;
        }
    }

    pub fn restore_from_vault(&mut self, saved: Option<&Secret>) -> SecretOrigin {
        let on_disk = self.holds_secrets();
        let Some(saved) = saved else {
            return if on_disk {
                SecretOrigin::Files
            } else {
                SecretOrigin::Missing
            };
        };
        let Ok(secrets) = EnvironmentSecrets::from_secret(saved) else {
            return SecretOrigin::Unreadable;
        };
        self.restore_secrets(&secrets);
        if on_disk {
            SecretOrigin::Files
        } else {
            SecretOrigin::Vault
        }
    }

    pub fn to_export_json(&self) -> Result<String, CoreError> {
        self.without_secret_values().to_json_pretty()
    }
}
