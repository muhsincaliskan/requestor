use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::CoreError;
use crate::text::scalar_text;

const ENVIRONMENT_SCOPE: &str = "environment";
const SECRET_KIND: &str = "secret";
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

    pub fn from_json(text: &str) -> Result<Self, CoreError> {
        Ok(serde_json::from_str(text)?)
    }

    pub fn to_json_pretty(&self) -> Result<String, CoreError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn value_of(&self, key: &str) -> Option<String> {
        self.values
            .iter()
            .find(|entry| entry.key == key && entry.is_enabled())
            .and_then(|entry| entry.value.as_ref())
            .and_then(scalar_text)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::EnvironmentValue;

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
