use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::param::Param;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BodyMode {
    Raw,
    Urlencoded,
    Formdata,
    File,
    Graphql,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Body {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<BodyMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urlencoded: Option<Vec<Param>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formdata: Option<Vec<Param>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graphql: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Body {
    pub fn is_enabled(&self) -> bool {
        !self.disabled.unwrap_or(false)
    }

    pub fn enabled_form_params(&self) -> Vec<&Param> {
        let params = match self.mode {
            Some(BodyMode::Urlencoded) => &self.urlencoded,
            Some(BodyMode::Formdata) => &self.formdata,
            _ => return Vec::new(),
        };
        params
            .iter()
            .flatten()
            .filter(|param| param.is_enabled())
            .collect()
    }
}
