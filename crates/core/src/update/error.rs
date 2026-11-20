use std::error::Error;
use std::fmt;

use crate::transport::TransportError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    Unreachable(TransportError),
    Status(u16),
    InvalidResponse,
    NoStableRelease,
    NoPackage,
    NoChecksums,
    ChecksumMismatch,
    Incomplete,
}

const NOT_FOUND: u16 = 404;

impl UpdateError {
    pub fn when_missing(self, missing: UpdateError) -> UpdateError {
        match self {
            UpdateError::Status(NOT_FOUND) => missing,
            other => other,
        }
    }
}

impl fmt::Display for UpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateError::Unreachable(error) => write!(formatter, "update check failed: {error}"),
            UpdateError::Status(status) => {
                write!(formatter, "update check failed with status {status}")
            }
            UpdateError::InvalidResponse => {
                write!(formatter, "update check returned an unexpected answer")
            }
            UpdateError::NoStableRelease => write!(formatter, "no stable release is published"),
            UpdateError::NoPackage => {
                write!(formatter, "the release has no package for this platform")
            }
            UpdateError::NoChecksums => write!(formatter, "the release has no checksum list"),
            UpdateError::ChecksumMismatch => {
                write!(
                    formatter,
                    "the downloaded package did not match its checksum"
                )
            }
            UpdateError::Incomplete => write!(formatter, "the download was cut short"),
        }
    }
}

impl Error for UpdateError {}
