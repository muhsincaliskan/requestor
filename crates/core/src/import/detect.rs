use serde_json::Value;

use super::{openapi, ImportError, Imported, ImportedCollection, ImportedRequest};
use crate::curl_command::COMMAND_NAME;

const URL_SCHEMES: [&str; 2] = ["http://", "https://"];
const MAX_OPENAPI_INPUT_BYTES: usize = 10 * 1024 * 1024;
const BYTES_PER_MEGABYTE: usize = 1024 * 1024;
const YAML_OPENAPI_MARKERS: [&str; 2] = ["openapi:", "swagger:"];

pub fn import_text(text: &str) -> Result<Imported, ImportError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ImportError::Empty);
    }
    if trimmed.starts_with('{') {
        return import_json(trimmed);
    }
    if trimmed.starts_with('[') {
        return Err(ImportError::UnsupportedFormat(
            "JSON arrays are not supported; expected a Postman collection, environment or OpenAPI object"
                .to_string(),
        ));
    }
    if is_curl_command(trimmed) {
        return ImportedRequest::from_curl(trimmed).map(Imported::Request);
    }
    if is_bare_url(trimmed) {
        return Ok(Imported::Request(ImportedRequest::from_url(trimmed)));
    }
    if YAML_OPENAPI_MARKERS
        .iter()
        .any(|marker| trimmed.starts_with(marker))
    {
        return Err(ImportError::UnsupportedFormat(
            "YAML OpenAPI documents are not supported; convert to OpenAPI 3 JSON".to_string(),
        ));
    }
    ImportedRequest::from_raw_http(trimmed).map(Imported::Request)
}

fn import_json(text: &str) -> Result<Imported, ImportError> {
    let document: Value =
        serde_json::from_str(text).map_err(|error| ImportError::InvalidJson(error.to_string()))?;
    if document.get("openapi").is_some() || document.get("swagger").is_some() {
        return import_openapi(text, &document);
    }
    if document.get("info").is_some() && document.get("item").is_some() {
        return serde_json::from_value(document)
            .map(|collection| {
                Imported::Collection(ImportedCollection::without_warnings(collection))
            })
            .map_err(|error| ImportError::InvalidJson(error.to_string()));
    }
    if document.get("values").is_some() && document.get("name").is_some() {
        return serde_json::from_value(document)
            .map(Imported::Environment)
            .map_err(|error| ImportError::InvalidJson(error.to_string()));
    }
    Err(ImportError::UnsupportedFormat(
        "the JSON document is not a Postman collection, environment or OpenAPI file".to_string(),
    ))
}

fn import_openapi(text: &str, document: &Value) -> Result<Imported, ImportError> {
    if text.len() > MAX_OPENAPI_INPUT_BYTES {
        return Err(ImportError::UnsupportedFormat(format!(
            "OpenAPI documents larger than {} MB are not supported",
            MAX_OPENAPI_INPUT_BYTES / BYTES_PER_MEGABYTE
        )));
    }
    openapi::from_json(document).map(Imported::Collection)
}

fn is_curl_command(text: &str) -> bool {
    text.split_whitespace().next() == Some(COMMAND_NAME)
}

fn is_bare_url(text: &str) -> bool {
    !text.contains(char::is_whitespace) && URL_SCHEMES.iter().any(|scheme| text.starts_with(scheme))
}
