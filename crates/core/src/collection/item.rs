use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::auth::Auth;
use super::event::Event;
use super::request::{Request, RequestSpec};
use super::variable::Variable;
use crate::text::Description;

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
    pub fn named(name: String) -> Self {
        Item {
            id: None,
            name: Some(name),
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

    pub fn for_request(name: String, request: Request) -> Self {
        Item {
            request: Some(RequestSpec::Full(Box::new(request))),
            ..Item::named(name)
        }
    }

    pub fn folder(name: String, items: Vec<Item>) -> Self {
        Item {
            item: Some(items),
            ..Item::named(name)
        }
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
