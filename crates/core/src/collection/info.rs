use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::text::Description;

pub const POSTMAN_COLLECTION_SCHEMA_V21: &str =
    "https://schema.getpostman.com/json/collection/v2.1.0/collection.json";
const SUPPORTED_SCHEMA_VERSION_MARKER: &str = "/v2.1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Info {
    pub name: String,
    #[serde(rename = "_postman_id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postman_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Info {
    pub fn is_supported_schema(schema: &str) -> bool {
        schema.contains(SUPPORTED_SCHEMA_VERSION_MARKER)
    }

    pub fn new(name: impl Into<String>) -> Self {
        Info {
            name: name.into(),
            postman_id: None,
            description: None,
            version: None,
            schema: Some(POSTMAN_COLLECTION_SCHEMA_V21.to_string()),
            extra: Map::new(),
        }
    }
}
