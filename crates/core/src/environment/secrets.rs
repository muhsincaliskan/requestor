use std::collections::BTreeMap;

use serde_json::Value;

use super::{Environment, EnvironmentValue};
use crate::error::CoreError;
use crate::secret::Secret;

impl EnvironmentValue {
    pub fn holds_secret(&self) -> bool {
        self.is_secret() && !self.value_text().is_empty()
    }
}

impl Environment {
    pub fn secret_values(&self) -> BTreeMap<String, Secret> {
        self.values
            .iter()
            .filter(|value| value.holds_secret())
            .map(|value| (value.key.clone(), Secret::new(value.value_text())))
            .collect()
    }

    pub fn declares_secrets(&self) -> bool {
        self.values.iter().any(EnvironmentValue::is_secret)
    }

    pub fn holds_secrets(&self) -> bool {
        self.values.iter().any(EnvironmentValue::holds_secret)
    }

    pub fn without_secret_values(&self) -> Environment {
        let mut stripped = self.clone();
        for value in stripped.values.iter_mut().filter(|value| value.is_secret()) {
            value.value = Some(Value::String(String::new()));
        }
        stripped
    }

    pub fn restore_secret_values(&mut self, saved: &BTreeMap<String, Secret>) {
        let blanks = self
            .values
            .iter_mut()
            .filter(|value| value.is_secret() && value.value_text().is_empty());
        for value in blanks {
            if let Some(secret) = saved.get(&value.key) {
                value.set_value_text(secret.expose());
            }
        }
    }

    pub fn to_export_json(&self) -> Result<String, CoreError> {
        self.without_secret_values().to_json_pretty()
    }
}
