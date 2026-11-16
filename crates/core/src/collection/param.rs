use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::text::{Description, StringOrList};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<StringOrList>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(rename = "contentType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Param {
    pub fn is_enabled(&self) -> bool {
        !self.disabled.unwrap_or(false)
    }
}

impl Param {
    pub fn query_pair(&self) -> String {
        let key = self.key.as_deref().unwrap_or_default();
        match &self.value {
            Some(value) => format!("{key}={value}"),
            None => key.to_string(),
        }
    }
}

impl Param {
    pub fn text(key: impl Into<String>, value: impl Into<String>) -> Self {
        Param {
            key: Some(key.into()),
            value: Some(value.into()),
            src: None,
            kind: None,
            content_type: None,
            disabled: None,
            description: None,
            extra: Map::new(),
        }
    }

    pub fn from_query_pair(pair: &str) -> Self {
        match pair.split_once('=') {
            Some((key, value)) => Param::text(key, value),
            None => Param {
                value: None,
                ..Param::text(pair, "")
            },
        }
    }

    pub fn file(key: impl Into<String>, path: impl Into<String>) -> Self {
        Param {
            value: None,
            src: Some(StringOrList::Single(path.into())),
            kind: Some("file".to_string()),
            ..Param::text(key, "")
        }
    }
}
