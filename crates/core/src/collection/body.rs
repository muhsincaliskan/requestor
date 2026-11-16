use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::param::Param;
use crate::media_type;

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

const RAW_LANGUAGE_CONTENT_TYPES: [(&str, &str); 5] = [
    ("json", media_type::JSON),
    ("xml", media_type::XML),
    ("html", media_type::HTML),
    ("javascript", media_type::JAVASCRIPT),
    ("text", media_type::PLAIN_TEXT),
];

impl Body {
    fn empty(mode: BodyMode) -> Self {
        Body {
            mode: Some(mode),
            raw: None,
            urlencoded: None,
            formdata: None,
            file: None,
            graphql: None,
            options: None,
            disabled: None,
            extra: Map::new(),
        }
    }

    pub fn raw(text: impl Into<String>, language: Option<&str>) -> Self {
        let options = language.map(|name| serde_json::json!({ "raw": { "language": name } }));
        Body {
            raw: Some(text.into()),
            options,
            ..Body::empty(BodyMode::Raw)
        }
    }

    pub fn urlencoded(params: Vec<Param>) -> Self {
        Body {
            urlencoded: Some(params),
            ..Body::empty(BodyMode::Urlencoded)
        }
    }

    pub fn formdata(params: Vec<Param>) -> Self {
        Body {
            formdata: Some(params),
            ..Body::empty(BodyMode::Formdata)
        }
    }

    pub fn file(path: impl Into<String>) -> Self {
        Body {
            file: Some(serde_json::json!({ "src": path.into() })),
            ..Body::empty(BodyMode::File)
        }
    }

    pub fn raw_content_type(&self) -> &'static str {
        let language = self
            .options
            .as_ref()
            .and_then(|options| options.get("raw")?.get("language")?.as_str());
        RAW_LANGUAGE_CONTENT_TYPES
            .iter()
            .find(|(name, _)| Some(*name) == language)
            .map_or(media_type::PLAIN_TEXT, |(_, content_type)| content_type)
    }

    pub fn file_source(&self) -> Option<&str> {
        self.file
            .as_ref()?
            .get("src")?
            .as_str()
            .filter(|path| !path.is_empty())
    }

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
