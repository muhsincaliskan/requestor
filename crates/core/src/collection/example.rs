use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::item::Item;
use super::param::Param;
use super::request::{Request, RequestSpec};
use crate::header::Header;
use crate::response::HttpResponse;

const UNNAMED_EXAMPLE: &str = "Example";
const EXAMPLE_VERSION: &str = "HTTP/1.1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Example {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "originalRequest")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_request: Option<RequestSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<Vec<Param>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Example {
    pub fn from_exchange(request: &Request, response: &HttpResponse) -> Self {
        let name = format!("{} {}", response.status, response.reason);
        Example {
            name: Some(name.trim().to_string()),
            original_request: Some(RequestSpec::Full(Box::new(request.clone()))),
            status: Some(response.reason.clone()),
            code: Some(response.status),
            header: Some(
                response
                    .headers
                    .iter()
                    .map(|header| Param::text(&header.name, &header.value))
                    .collect(),
            ),
            body: Some(String::from_utf8_lossy(&response.body).into_owned()),
            extra: Map::new(),
        }
    }

    pub fn display_name(&self) -> &str {
        self.name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(UNNAMED_EXAMPLE)
    }

    pub fn request(&self) -> Option<Request> {
        self.original_request.as_ref().map(RequestSpec::to_request)
    }

    pub fn to_response(&self) -> HttpResponse {
        HttpResponse {
            status: self.code.unwrap_or_default(),
            reason: self.status.clone().unwrap_or_default(),
            version: EXAMPLE_VERSION.to_string(),
            headers: self
                .header
                .iter()
                .flatten()
                .filter(|param| param.is_enabled())
                .map(|param| Header {
                    name: param.key.clone().unwrap_or_default(),
                    value: param.value.clone().unwrap_or_default(),
                })
                .collect(),
            body: self.body.clone().unwrap_or_default().into_bytes(),
            truncated: false,
            elapsed: Duration::ZERO,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ExampleSummary {
    #[serde(skip)]
    pub index: usize,
    name: Option<String>,
    pub code: Option<u16>,
}

impl ExampleSummary {
    pub fn display_name(&self) -> &str {
        self.name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(UNNAMED_EXAMPLE)
    }
}

impl Item {
    pub fn example_summaries(&self) -> impl Iterator<Item = ExampleSummary> + '_ {
        self.response
            .iter()
            .flatten()
            .enumerate()
            .filter_map(|(index, value)| {
                ExampleSummary::deserialize(value)
                    .ok()
                    .map(|summary| ExampleSummary { index, ..summary })
            })
    }

    pub fn example(&self, index: usize) -> Option<Example> {
        Example::deserialize(self.response.as_ref()?.get(index)?).ok()
    }

    pub fn add_example(&mut self, example: &Example) {
        if let Ok(value) = serde_json::to_value(example) {
            self.response.get_or_insert_with(Vec::new).push(value);
        }
    }

    pub fn remove_example(&mut self, index: usize) -> bool {
        let Some(examples) = self.response.as_mut() else {
            return false;
        };
        if index >= examples.len() {
            return false;
        }
        examples.remove(index);
        if examples.is_empty() {
            self.response = None;
        }
        true
    }
}
