use std::error::Error;
use std::fmt;

use crate::transport::TransportError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    Unreachable(TransportError),
    Status(u16),
    InvalidResponse,
    NoStableRelease,
    ChecksumMismatch,
    Incomplete,
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
