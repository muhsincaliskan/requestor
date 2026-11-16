use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::CoreError;
use crate::text::scalar_text;

const ENVIRONMENT_SCOPE: &str = "environment";

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
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
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
