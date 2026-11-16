mod curl;
mod detect;
mod raw_http;
mod shell_words;

pub mod openapi;

use std::fmt;

use crate::collection::{Collection, Request, Url, UrlParts, SCHEME_SEPARATOR};
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

impl ImportedRequest {
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
