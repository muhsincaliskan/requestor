mod auth;
mod body;
mod event;
mod info;
mod item;
mod param;
mod request;
mod url;
mod variable;

pub use auth::{Auth, AuthAttribute};
pub use body::{Body, BodyMode};
pub use event::{Event, Script};
pub use info::{Info, POSTMAN_COLLECTION_SCHEMA_V21};
pub use item::Item;
pub use param::Param;
pub use request::{Request, RequestSpec};
pub use url::{Url, UrlParts};
pub use variable::Variable;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::CoreError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub info: Info,
    pub item: Vec<Item>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<Vec<Event>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<Vec<Variable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Collection {
    pub fn new(name: impl Into<String>) -> Self {
        Collection {
            info: Info::new(name),
            item: Vec::new(),
            event: None,
            variable: None,
            auth: None,
            extra: Map::new(),
        }
    }

    pub fn from_json(text: &str) -> Result<Self, CoreError> {
        Ok(serde_json::from_str(text)?)
    }

    pub fn to_json_pretty(&self) -> Result<String, CoreError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn item_at(&self, path: &[usize]) -> Option<&Item> {
        let (first, rest) = path.split_first()?;
        rest.iter().try_fold(self.item.get(*first)?, |item, index| {
            item.children().get(*index)
        })
    }

    pub fn item_at_mut(&mut self, path: &[usize]) -> Option<&mut Item> {
        let (first, rest) = path.split_first()?;
        let mut current = self.item.get_mut(*first)?;
        for index in rest {
            current = current.item.as_mut()?.get_mut(*index)?;
        }
        Some(current)
    }

    pub fn children_at_mut(&mut self, parent: &[usize]) -> Option<&mut Vec<Item>> {
        if parent.is_empty() {
            return Some(&mut self.item);
        }
        self.item_at_mut(parent)?.item.as_mut()
    }

    pub fn add_item(&mut self, parent: &[usize], item: Item) -> Option<Vec<usize>> {
        let children = self.children_at_mut(parent)?;
        children.push(item);
        let mut path = parent.to_vec();
        path.push(children.len() - 1);
        Some(path)
    }

    pub fn remove_item(&mut self, path: &[usize]) -> Option<Item> {
        let (index, parent) = path.split_last()?;
        let children = self.children_at_mut(parent)?;
        (*index < children.len()).then(|| children.remove(*index))
    }

    pub fn request_count(&self) -> usize {
        self.item.iter().map(Item::request_count).sum()
    }
}
