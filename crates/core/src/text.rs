use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrList {
    Single(String),
    List(Vec<String>),
}

impl StringOrList {
    pub fn joined(&self, separator: &str) -> String {
        match self {
            StringOrList::Single(text) => text.clone(),
            StringOrList::List(parts) => parts.join(separator),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Description {
    Text(String),
    Rich {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(rename = "type")]
        #[serde(skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<Value>,
        #[serde(flatten)]
        extra: Map<String, Value>,
    },
}

impl Description {
    pub fn content(&self) -> Option<&str> {
        match self {
            Description::Text(text) => Some(text),
            Description::Rich { content, .. } => content.as_deref(),
        }
    }
}

pub fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}
