mod curl;
mod detect;
mod raw_http;
mod shell_words;

pub mod openapi;

use std::fmt;

use crate::collection::{Collection, Request};
use crate::environment::Environment;

pub use detect::import_text;

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
pub enum Imported {
    Collection(Collection),
    Environment(Environment),
    Request(ImportedRequest),
}
