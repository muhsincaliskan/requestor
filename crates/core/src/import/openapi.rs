use serde_json::Value;

use super::ImportError;
use crate::collection::Collection;

pub fn from_json(_document: &Value) -> Result<Collection, ImportError> {
    Err(ImportError::UnsupportedFormat(
        "OpenAPI import is not implemented yet".to_string(),
    ))
}
