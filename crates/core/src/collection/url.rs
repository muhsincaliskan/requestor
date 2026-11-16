use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::param::Param;
use super::variable::Variable;
use crate::text::StringOrList;

const HOST_SEPARATOR: &str = ".";
const PATH_SEPARATOR: &str = "/";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrlParts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<StringOrList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<StringOrList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<Vec<Param>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<Vec<Variable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Url {
    Raw(String),
    Parts(Box<UrlParts>),
}

impl Url {
    pub fn to_text(&self) -> String {
        match self {
            Url::Raw(text) => text.clone(),
            Url::Parts(parts) => parts.to_text(),
        }
    }
}

impl UrlParts {
    pub fn to_text(&self) -> String {
        if let Some(raw) = &self.raw {
            return raw.clone();
        }
        let host = self
            .host
            .as_ref()
            .map(|host| host.joined(HOST_SEPARATOR))
            .unwrap_or_default();
        let path = self
            .path
            .as_ref()
            .map(|path| path.joined(PATH_SEPARATOR))
            .unwrap_or_default();
        let port = self.port.as_ref().map(|port| format!(":{port}"));
        let scheme = self.protocol.as_ref().map(|name| format!("{name}://"));
        format!(
            "{}{host}{}/{path}",
            scheme.unwrap_or_default(),
            port.unwrap_or_default()
        )
    }
}
