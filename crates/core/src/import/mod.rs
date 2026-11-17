mod curl;
mod detect;
mod raw_http;
mod shell_words;

pub mod openapi;

use std::fmt;

use crate::collection::{Collection, Item, Request, Url, UrlParts, SCHEME_SEPARATOR};
use crate::environment::Environment;

pub use detect::import_text;

const DEFAULT_SCHEME: &str = "http";
const GET_METHOD: &str = "GET";

#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    Empty,
    UnterminatedQuote,
    MissingUrl,
    InvalidRequestLine(String),
    InvalidJson(String),
    UnsupportedFormat(String),
}

impl fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::Empty => write!(formatter, "nothing to import"),
            ImportError::UnterminatedQuote => {
                write!(formatter, "the command has an unterminated quote")
            }
            ImportError::MissingUrl => write!(formatter, "no URL found"),
            ImportError::InvalidRequestLine(line) => {
                write!(formatter, "not a valid request line: {line}")
            }
            ImportError::InvalidJson(reason) => write!(formatter, "invalid JSON: {reason}"),
            ImportError::UnsupportedFormat(reason) => {
                write!(formatter, "unsupported format: {reason}")
            }
        }
    }
}

impl std::error::Error for ImportError {}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedRequest {
    pub name: String,
    pub request: Request,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedCollection {
    pub collection: Collection,
    pub warnings: Vec<String>,
}

impl ImportedCollection {
    pub fn without_warnings(collection: Collection) -> Self {
        ImportedCollection {
            collection,
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Imported {
    Collection(ImportedCollection),
    Environment(Environment),
    Request(ImportedRequest),
}

impl Imported {
    pub fn warnings(&self) -> &[String] {
        match self {
            Imported::Collection(imported) => &imported.warnings,
            Imported::Request(imported) => &imported.warnings,
            Imported::Environment(_) => &[],
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Imported::Collection(_) => "a collection",
            Imported::Environment(_) => "an environment",
            Imported::Request(_) => "a request",
        }
    }
}

impl TryFrom<Imported> for ImportedRequest {
    type Error = ImportError;

    fn try_from(imported: Imported) -> Result<Self, Self::Error> {
        match imported {
            Imported::Request(request) => Ok(request),
            other => Err(ImportError::UnsupportedFormat(format!(
                "expected a single request but found {}",
                other.kind()
            ))),
        }
    }
}

impl ImportedRequest {
    pub fn into_item(self) -> Item {
        Item::for_request(self.name, self.request)
    }

    pub fn into_collection(self) -> Collection {
        let mut collection = Collection::new(self.name.clone());
        collection.item.push(self.into_item());
        collection
    }

    pub fn from_url(url: &str) -> Self {
        ImportedRequest::new(GET_METHOD, url)
    }

    fn new(method: &str, url: &str) -> Self {
        let url = if Url::split_scheme(url).is_some() {
            url.to_string()
        } else {
            format!("{DEFAULT_SCHEME}{SCHEME_SEPARATOR}{url}")
        };
        ImportedRequest {
            name: format!("{method} {}", display_path(&url)),
            request: Request::new(method, Url::Parts(Box::new(UrlParts::parse(&url)))),
            warnings: Vec::new(),
        }
    }
}

fn display_path(url: &str) -> String {
    let without_scheme = Url::split_scheme(url).map_or(url, |(_, rest)| rest);
    without_scheme
        .split(['?', '#'])
        .next()
        .unwrap_or(without_scheme)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{import_text, ImportError, ImportedRequest};

    #[test]
    fn a_request_converts_and_other_documents_name_what_was_found() {
        let request = import_text("curl http://h/x").unwrap();
        assert!(ImportedRequest::try_from(request).is_ok());
        let environment = import_text(r#"{"name":"E","values":[]}"#).unwrap();
        match ImportedRequest::try_from(environment) {
            Err(ImportError::UnsupportedFormat(reason)) => {
                assert!(reason.contains("an environment"), "{reason}");
            }
            other => panic!("unexpected {other:?}"),
        }
        let collection = import_text(r#"{"info":{"name":"c"},"item":[]}"#).unwrap();
        assert!(ImportedRequest::try_from(collection).is_err());
    }

    #[test]
    fn warnings_are_reachable_from_every_kind() {
        let request = import_text("curl http://h/x").unwrap();
        assert!(request.warnings().is_empty());
        let environment = import_text(r#"{"name":"E","values":[]}"#).unwrap();
        assert!(environment.warnings().is_empty());
    }
}
