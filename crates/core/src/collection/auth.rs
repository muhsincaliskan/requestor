use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const NO_AUTH_KIND: &str = "noauth";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthAttribute {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Auth {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub parameters: BTreeMap<String, Option<Vec<AuthAttribute>>>,
}

impl Auth {
    pub fn value_of(&self, key: &str) -> Option<&str> {
        self.parameters
            .get(&self.kind)?
            .as_ref()?
            .iter()
            .find(|attribute| attribute.key.as_deref() == Some(key))
            .and_then(|attribute| attribute.value.as_ref())
            .and_then(Value::as_str)
    }

    pub fn is_none(&self) -> bool {
        self.kind == NO_AUTH_KIND
    }
}
