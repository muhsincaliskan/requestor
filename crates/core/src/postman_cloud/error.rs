use std::error::Error;
use std::fmt;

use crate::transport::TransportError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostmanCloudError {
    Unreachable(TransportError),
    KeyRejected,
    NotFound,
    RateLimited,
    Status(u16),
    InvalidResponse(String),
    InvalidIdentifier(String),
    TooLarge,
    Setup(String),
}

impl PostmanCloudError {
    pub fn stops_migration(&self) -> bool {
        matches!(
            self,
            PostmanCloudError::KeyRejected | PostmanCloudError::RateLimited
        )
    }
}

impl fmt::Display for PostmanCloudError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PostmanCloudError::Unreachable(error) => {
                write!(formatter, "could not reach Postman: {error}")
            }
            PostmanCloudError::KeyRejected => {
                write!(formatter, "Postman rejected the API key")
            }
            PostmanCloudError::NotFound => write!(formatter, "Postman could not find it"),
            PostmanCloudError::RateLimited => write!(
                formatter,
                "Postman's API limit was reached; try again later"
            ),
            PostmanCloudError::Status(status) => {
                write!(formatter, "Postman answered with status {status}")
            }
            PostmanCloudError::InvalidResponse(reason) => {
                write!(formatter, "Postman sent an unexpected answer: {reason}")
            }
            PostmanCloudError::TooLarge => {
                write!(formatter, "it is larger than Requestor imports at once")
            }
            PostmanCloudError::Setup(reason) => {
                write!(formatter, "could not prepare the connection: {reason}")
            }
            PostmanCloudError::InvalidIdentifier(id) => {
                write!(formatter, "Postman sent an unusable identifier: {id}")
            }
        }
    }
}

impl Error for PostmanCloudError {}
