pub mod auth;
mod body;
mod browse;
mod event;
mod identity;
mod info;
mod item;
mod param;
mod path_variables;
mod request;
mod run;
mod tree;
mod url;
mod variable;

pub use auth::{Auth, AuthAttribute};
pub(crate) use body::JSON_OPENERS;
pub use body::{Body, BodyMode};
pub use browse::RequestLocation;
pub use event::{Event, Script};
pub use info::{Info, POSTMAN_COLLECTION_SCHEMA_V21};
pub use item::Item;
pub use param::Param;
pub(crate) use param::{FILE_KIND, STRING_KIND, TEXT_KIND};
pub use path_variables::path_variable_names;
pub use request::{Request, RequestSpec};
pub use run::RunnableRequest;
pub use url::{Url, UrlParts, SCHEME_SEPARATOR};
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

    pub fn to_export_json(&self) -> Result<String, CoreError> {
        let mut stripped = self.clone();
        stripped
            .variable
            .iter_mut()
            .flatten()
            .for_each(Variable::clear_secret_value);
        stripped.item.iter_mut().for_each(Item::clear_secret_values);
        stripped.to_json_pretty()
    }

    pub fn request_count(&self) -> usize {
        self.item.iter().map(Item::request_count).sum()
    }
}
