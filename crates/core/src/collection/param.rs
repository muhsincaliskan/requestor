use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::encoding::{percent_decode, percent_encode};
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

    pub fn encode_form_pair(key: &str, value: &str) -> String {
        if key.is_empty() {
            percent_encode(value)
        } else {
            format!("{}={}", percent_encode(key), percent_encode(value))
        }
    }

    pub fn form_pair(&self) -> String {
        Param::encode_form_pair(
            self.key.as_deref().unwrap_or_default(),
            self.value.as_deref().unwrap_or_default(),
        )
    }

    pub fn from_form_text(text: &str) -> Option<Vec<Param>> {
        text.split('&')
            .map(|pair| {
                pair.split_once('=')
                    .map(|(key, value)| Param::text(percent_decode(key), percent_decode(value)))
            })
            .collect()
    }

    pub fn file_with_type(
        key: impl Into<String>,
        path: impl Into<String>,
        content_type: Option<&str>,
    ) -> Self {
        Param {
            content_type: content_type.map(str::to_string),
            ..Param::file(key, path)
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

#[cfg(test)]
mod tests {
    use super::Param;

    #[test]
    fn form_pairs_encode_once_and_omit_the_equals_for_empty_keys() {
        assert_eq!(Param::text("q", "a b&c").form_pair(), "q=a%20b%26c");
        assert_eq!(Param::text("", "a b").form_pair(), "a%20b");
    }

    #[test]
    fn form_text_decodes_pairs_and_rejects_bare_words() {
        let params = Param::from_form_text("a=1%202&b=").unwrap();
        assert_eq!(params[0].value.as_deref(), Some("1 2"));
        assert!(Param::from_form_text("plain").is_none());
    }
}
