use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::CoreError;
use crate::identity::random_uuid;
use crate::text::scalar_text;

const ENVIRONMENT_SCOPE: &str = "environment";
pub(crate) const SECRET_KIND: &str = "secret";
const DEFAULT_KIND: &str = "default";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentValue {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl EnvironmentValue {
    pub fn blank() -> Self {
        EnvironmentValue {
            key: String::new(),
            value: Some(Value::String(String::new())),
            kind: Some(DEFAULT_KIND.to_string()),
            enabled: Some(true),
            extra: Map::new(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = Some(enabled);
    }

    pub fn is_secret(&self) -> bool {
        self.kind.as_deref() == Some(SECRET_KIND)
    }

    pub fn set_secret(&mut self, secret: bool) {
        let kind = if secret { SECRET_KIND } else { DEFAULT_KIND };
        self.kind = Some(kind.to_string());
    }

    pub fn value_text(&self) -> String {
        self.value
            .as_ref()
            .and_then(scalar_text)
            .unwrap_or_default()
    }

    pub fn set_value_text(&mut self, text: &str) {
        if self.value_text() != text {
            self.value = Some(Value::String(text.to_string()));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Environment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub values: Vec<EnvironmentValue>,
    #[serde(rename = "_postman_variable_scope")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_scope: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Environment {
    pub fn new(name: impl Into<String>) -> Self {
        Environment {
            id: None,
            name: name.into(),
            values: Vec::new(),
            variable_scope: Some(ENVIRONMENT_SCOPE.to_string()),
            extra: Map::new(),
        }
    }

    pub fn ensure_id(&mut self) -> bool {
        let missing = self.id.is_none();
        if missing {
            self.id = Some(random_uuid());
        }
        missing
    }

    pub fn claim_unique_id(&mut self, taken: &mut HashSet<String>) -> bool {
        let mut changed = self.ensure_id();
        let id = self.id.clone().unwrap_or_default();
        if !taken.insert(id) {
            let fresh = random_uuid();
            taken.insert(fresh.clone());
            self.id = Some(fresh);
            changed = true;
        }
        changed
    }

    pub fn from_json(text: &str) -> Result<Self, CoreError> {
        Ok(serde_json::from_str(text)?)
    }

    pub fn to_json_pretty(&self) -> Result<String, CoreError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn set_value(&mut self, key: &str, text: &str) {
        let index = match self.values.iter().position(|entry| entry.key == key) {
            Some(index) => index,
            None => {
                self.values.push(EnvironmentValue {
                    key: key.to_string(),
                    ..EnvironmentValue::blank()
                });
                self.values.len() - 1
            }
        };
        let entry = &mut self.values[index];
        entry.set_value_text(text);
        entry.set_enabled(true);
    }

    pub fn value_of(&self, key: &str) -> Option<String> {
        self.values
            .iter()
            .find(|entry| entry.key == key && entry.is_enabled())
            .and_then(|entry| entry.value.as_ref())
            .and_then(scalar_text)
    }
}

mod marking;
mod secrets;

pub use marking::{EnvironmentColor, PRODUCTION_NAMES};

#[cfg(test)]
mod tests {
    use serde_json::json;

    use std::collections::HashSet;

    use super::{Environment, EnvironmentValue};

    #[test]
    fn ensure_id_names_an_environment_once() {
        let mut environment = Environment::new("Staging");
        assert!(environment.ensure_id());
        let id = environment.id.clone();
        assert!(!environment.ensure_id());
        assert_eq!(environment.id, id);
    }

    #[test]
    fn claiming_a_taken_id_gives_a_fresh_one() {
        let mut taken = HashSet::new();
        let mut first = Environment::new("A");
        assert!(first.claim_unique_id(&mut taken));
        let mut second = Environment::new("B");
        second.id.clone_from(&first.id);
        assert!(second.claim_unique_id(&mut taken));
        assert_ne!(second.id, first.id);
        assert!(!first.claim_unique_id(&mut HashSet::new()));
    }

    #[test]
    fn secret_flag_switches_the_kind() {
        let mut entry = EnvironmentValue::blank();
        assert!(!entry.is_secret());
        entry.set_secret(true);
        assert!(entry.is_secret());
        assert_eq!(entry.kind.as_deref(), Some("secret"));
        entry.set_secret(false);
        assert_eq!(entry.kind.as_deref(), Some("default"));
    }

    #[test]
    fn untouched_non_string_values_are_preserved() {
        let mut entry = EnvironmentValue::blank();
        entry.value = Some(json!(true));
        assert_eq!(entry.value_text(), "true");
        entry.set_value_text("true");
        assert_eq!(entry.value, Some(json!(true)));
        entry.set_value_text("no");
        assert_eq!(entry.value, Some(json!("no")));
    }

    #[test]
    fn missing_values_read_as_empty_text_and_enabled_is_explicit() {
        let mut entry = EnvironmentValue::blank();
        entry.value = None;
        assert_eq!(entry.value_text(), "");
        entry.set_enabled(false);
        assert!(!entry.is_enabled());
    }
}
