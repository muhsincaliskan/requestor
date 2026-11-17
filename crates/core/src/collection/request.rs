use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::auth::Auth;
use super::body::Body;
use super::param::Param;
use super::url::Url;
use crate::method::HttpMethod;
use crate::text::Description;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<Vec<Param>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Body>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestSpec {
    Url(String),
    Full(Box<Request>),
}

impl Request {
    pub fn new(method: &str, url: Url) -> Self {
        Request {
            method: Some(method.to_string()),
            header: None,
            body: None,
            url: Some(url),
            auth: None,
            description: None,
            extra: Map::new(),
        }
    }

    pub fn push_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.header
            .get_or_insert_with(Vec::new)
            .push(Param::text(name, value));
    }

    pub fn has_header(&self, name: &str) -> bool {
        self.header.iter().flatten().any(|header| {
            header
                .key
                .as_deref()
                .is_some_and(|key| key.eq_ignore_ascii_case(name))
        })
    }

    pub fn header_value(&self, name: &str) -> Option<&str> {
        self.header
            .iter()
            .flatten()
            .find(|header| {
                header
                    .key
                    .as_deref()
                    .is_some_and(|key| key.eq_ignore_ascii_case(name))
            })?
            .value
            .as_deref()
    }

    pub fn http_method(&self) -> HttpMethod {
        self.method
            .as_deref()
            .map(HttpMethod::parse)
            .unwrap_or_default()
    }

    pub fn enabled_headers(&self) -> Vec<&Param> {
        self.header
            .iter()
            .flatten()
            .filter(|header| header.is_enabled())
            .collect()
    }
}

impl RequestSpec {
    pub fn to_request(&self) -> Request {
        match self {
            RequestSpec::Full(request) => (**request).clone(),
            RequestSpec::Url(text) => Request {
                method: None,
                header: None,
                body: None,
                url: Some(Url::Raw(text.clone())),
                auth: None,
                description: None,
                extra: Map::new(),
            },
        }
    }
}
