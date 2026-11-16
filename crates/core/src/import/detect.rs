use serde_json::Value;

use super::{openapi, ImportError, Imported, ImportedRequest};
use crate::collection::Collection;
use crate::environment::Environment;

const CURL_COMMAND: &str = "curl";
const URL_SCHEMES: [&str; 2] = ["http://", "https://"];
const YAML_OPENAPI_MARKERS: [&str; 2] = ["openapi:", "swagger:"];

pub fn import_text(text: &str) -> Result<Imported, ImportError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ImportError::Empty);
    }
    if trimmed.starts_with('{') {
        return import_json(trimmed);
    }
    if is_curl_command(trimmed) {
        return ImportedRequest::from_curl(trimmed).map(Imported::Request);
    }
    if is_bare_url(trimmed) {
        return ImportedRequest::from_curl(&format!("{CURL_COMMAND} '{trimmed}'"))
            .map(Imported::Request);
    }
    if YAML_OPENAPI_MARKERS
        .iter()
        .any(|marker| trimmed.starts_with(marker))
    {
        return Err(ImportError::UnsupportedFormat(
            "YAML OpenAPI documents are not supported yet; convert the file to JSON".to_string(),
        ));
    }
    ImportedRequest::from_raw_http(trimmed).map(Imported::Request)
}

fn import_json(text: &str) -> Result<Imported, ImportError> {
    let document: Value =
        serde_json::from_str(text).map_err(|error| ImportError::InvalidJson(error.to_string()))?;
    if document.get("openapi").is_some() || document.get("swagger").is_some() {
        return openapi::from_json(&document).map(Imported::Collection);
    }
    if document.get("info").is_some() && document.get("item").is_some() {
        return Collection::from_json(text)
            .map(Imported::Collection)
            .map_err(|error| ImportError::InvalidJson(error.to_string()));
    }
    if document.get("values").is_some() && document.get("name").is_some() {
        return Environment::from_json(text)
            .map(Imported::Environment)
            .map_err(|error| ImportError::InvalidJson(error.to_string()));
    }
    Err(ImportError::UnsupportedFormat(
        "the JSON document is not a Postman collection, environment or OpenAPI file".to_string(),
    ))
}

fn is_curl_command(text: &str) -> bool {
    text.split_whitespace().next() == Some(CURL_COMMAND)
}

fn is_bare_url(text: &str) -> bool {
    !text.contains(char::is_whitespace) && URL_SCHEMES.iter().any(|scheme| text.starts_with(scheme))
}
