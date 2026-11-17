use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::param::Param;
use crate::media_type;
use crate::soap::SoapVersion;

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

pub(crate) const JSON_OPENERS: [char; 2] = ['{', '['];
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

    pub fn raw_for_content_type(text: impl Into<String>, content_type: Option<&str>) -> Self {
        let language = content_type.and_then(Body::language_of);
        Body::raw(text, language)
    }

    pub fn from_data(data: &str, content_type: Option<&str>) -> Self {
        match content_type {
            Some(kind) if !Body::is_form_content_type(kind) => {
                Body::raw_for_content_type(data, Some(kind))
            }
            None if Body::looks_like_json(data) => Body::raw(data, None),
            _ => {
                Param::from_form_text(data).map_or_else(|| Body::raw(data, None), Body::urlencoded)
            }
        }
    }

    fn looks_like_json(data: &str) -> bool {
        data.trim_start().starts_with(JSON_OPENERS)
    }

    pub fn is_form_content_type(content_type: &str) -> bool {
        content_type
            .to_ascii_lowercase()
            .contains(media_type::FORM_URLENCODED)
    }

    fn language_of(content_type: &str) -> Option<&'static str> {
        let lowered = content_type.to_ascii_lowercase();
        RAW_LANGUAGE_CONTENT_TYPES
            .iter()
            .map(|(name, _)| *name)
            .find(|name| lowered.contains(name))
    }

    pub fn raw_content_type(&self) -> &'static str {
        if let Some(version) = self.raw.as_deref().and_then(SoapVersion::of_envelope) {
            return version.content_type();
        }
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

#[cfg(test)]
mod tests {
    use super::Body;
    use crate::media_type;
    use crate::soap::SoapVersion;

    #[test]
    fn raw_language_follows_the_content_type() {
        for (content_type, expected) in [
            ("application/vnd.api+json; charset=utf-8", media_type::JSON),
            ("text/xml", media_type::XML),
            ("text/html", media_type::HTML),
            ("text/plain", media_type::PLAIN_TEXT),
            ("application/octet-stream", media_type::PLAIN_TEXT),
        ] {
            let body = Body::raw_for_content_type("x", Some(content_type));
            assert_eq!(body.raw_content_type(), expected, "{content_type}");
        }
    }

    #[test]
    fn soap_envelopes_are_sent_with_their_version_content_type() {
        for version in SoapVersion::ALL {
            let body = Body::raw(version.envelope(""), Some("xml"));
            assert_eq!(body.raw_content_type(), version.content_type());
        }
    }

    #[test]
    fn data_without_a_form_content_type_stays_raw() {
        assert!(Body::from_data("a=1", Some("text/plain")).raw.is_some());
        assert!(Body::from_data("a=1&b=%20", None).urlencoded.is_some());
        assert!(Body::from_data("not pairs", None).raw.is_some());
    }
}
