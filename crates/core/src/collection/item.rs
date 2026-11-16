use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::auth::Auth;
use super::event::Event;
use super::request::{Request, RequestSpec};
use super::variable::Variable;
use crate::text::Description;

const UNNAMED_ITEM: &str = "Untitled";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<Vec<Variable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<Vec<Event>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<Vec<Item>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<RequestSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Vec<Value>>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Item {
    pub fn request_item(name: impl Into<String>, request: Request) -> Self {
        Item {
            name: Some(name.into()),
            request: Some(RequestSpec::Full(Box::new(request))),
            response: Some(Vec::new()),
            ..Item::blank()
        }
    }

    pub fn folder(name: impl Into<String>) -> Self {
        Item {
            name: Some(name.into()),
            item: Some(Vec::new()),
            ..Item::blank()
        }
    }

    fn blank() -> Self {
        Item {
            id: None,
            name: None,
            description: None,
            variable: None,
            event: None,
            auth: None,
            item: None,
            request: None,
            response: None,
            extra: Map::new(),
        }
    }

    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(UNNAMED_ITEM)
    }

    pub fn children(&self) -> &[Item] {
        self.item.as_deref().unwrap_or_default()
    }

    pub fn full_request(&self) -> Option<Request> {
        self.request.as_ref().map(RequestSpec::to_request)
    }

    pub fn replace_request(&mut self, request: Request) {
        self.request = Some(RequestSpec::Full(Box::new(request)));
    }

    pub fn is_folder(&self) -> bool {
        self.item.is_some()
    }

    pub fn request_count(&self) -> usize {
        let own = usize::from(self.request.is_some());
        let nested: usize = self.item.iter().flatten().map(Item::request_count).sum();
        own + nested
    }
}
