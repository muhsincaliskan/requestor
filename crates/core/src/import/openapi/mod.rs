mod body;
mod document;
mod example;
mod operation;
mod parameter;
mod reference;
mod security;
mod tree;
mod values;

use serde_json::Value;

use super::{ImportError, ImportedCollection};
use document::Document;
use reference::Resolver;

pub const MAX_OPERATIONS: usize = 2000;
pub const BASE_URL_VARIABLE: &str = "baseUrl";
pub const BASE_URL_PLACEHOLDER: &str = "{{baseUrl}}";

pub fn from_json(root: &Value) -> Result<ImportedCollection, ImportError> {
    let document = Document::parse(root)?;
    let mut resolver = Resolver::new(document.root());
    let collection = document.to_collection(&mut resolver);
    Ok(ImportedCollection {
        collection,
        warnings: resolver.into_warnings(),
    })
}
