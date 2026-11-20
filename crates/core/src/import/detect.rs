use serde_json::Value;

use std::path::Path;

use super::wsdl::{self, DocumentSource, FileSource, PastedSource, RemoteSource};
use super::{is_web_address, openapi, ImportError, Imported, ImportedCollection, ImportedRequest};
use crate::collection::Info;
use crate::settings::RequestSettings;
use crate::settings::BYTES_PER_MEGABYTE;
use crate::transport::{DocumentTransport, HttpTransport};

const COLLECTION_SCHEMA_POINTER: &str = "/info/schema";
pub const MAX_IMPORT_FILE_BYTES: u64 = 64 * BYTES_PER_MEGABYTE as u64;
const MAX_OPENAPI_INPUT_BYTES: usize = 10 * 1024 * 1024;
const OPENAPI_KEY_MARKERS: [&str; 2] = ["\"openapi\"", "\"swagger\""];
const YAML_OPENAPI_MARKERS: [&str; 2] = ["openapi:", "swagger:"];

const PASTED_LOCATION: &str = "";
const BYTE_ORDER_MARK: char = '\u{feff}';

pub fn import_text(text: &str) -> Result<Imported, ImportError> {
    import_from(text, PASTED_LOCATION, &PastedSource)
}

pub fn import_file_text(text: &str, path: &Path) -> Result<Imported, ImportError> {
    import_from(text, &path.to_string_lossy(), &FileSource)
}

pub fn download_address(
    address: &str,
    settings: &RequestSettings,
) -> Result<Imported, ImportError> {
    let transport = DocumentTransport::new(settings, wsdl::MAX_DOCUMENT_BYTES)
        .map_err(|error| ImportError::Download(error.to_string()))?;
    import_address(address, &transport)
}

pub fn import_address(
    address: &str,
    transport: &dyn HttpTransport,
) -> Result<Imported, ImportError> {
    let source = RemoteSource::new(transport);
    let address = address.trim();
    let text = source.read(address).map_err(ImportError::Download)?;
    import_from(&text, address, &source)
}

fn import_from(
    text: &str,
    location: &str,
    source: &dyn DocumentSource,
) -> Result<Imported, ImportError> {
    let trimmed = text.trim_start_matches(BYTE_ORDER_MARK).trim();
    if trimmed.is_empty() {
        return Err(ImportError::Empty);
    }
    if wsdl::is_xml(trimmed) {
        return wsdl::from_xml(trimmed, location, source).map(Imported::Collection);
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
    if ImportedRequest::is_curl_command(trimmed) {
        return ImportedRequest::from_curl(trimmed).map(Imported::Request);
    }
    if is_web_address(trimmed) {
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

fn looks_like_openapi(text: &str) -> bool {
    OPENAPI_KEY_MARKERS
        .iter()
        .any(|marker| text.contains(marker))
}

fn import_json(text: &str) -> Result<Imported, ImportError> {
    if text.len() > MAX_OPENAPI_INPUT_BYTES && looks_like_openapi(text) {
        return Err(ImportError::UnsupportedFormat(format!(
            "OpenAPI documents larger than {} MB are not supported",
            MAX_OPENAPI_INPUT_BYTES / BYTES_PER_MEGABYTE
        )));
    }
    let document: Value =
        serde_json::from_str(text).map_err(|error| ImportError::InvalidJson(error.to_string()))?;
    if document.get("openapi").is_some() || document.get("swagger").is_some() {
        return openapi::from_json(&document).map(Imported::Collection);
    }
    if document.get("info").is_some() && document.get("item").is_some() {
        return import_collection(document);
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

fn import_collection(document: Value) -> Result<Imported, ImportError> {
    let declared_schema = document
        .pointer(COLLECTION_SCHEMA_POINTER)
        .and_then(Value::as_str)
        .map(str::to_string);
    serde_json::from_value(document)
        .map(|collection| Imported::Collection(ImportedCollection::without_warnings(collection)))
        .map_err(|error| match declared_schema {
            Some(schema) if !Info::is_supported_schema(&schema) => {
                ImportError::UnsupportedFormat(format!(
                    "only Postman Collection v2.1 is supported, but this file declares {schema}"
                ))
            }
            _ => ImportError::InvalidJson(error.to_string()),
        })
}
