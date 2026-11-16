use std::error::Error;
use std::fmt;

const TLS_MARKERS: [&str; 3] = ["certificate", "tls", "handshake"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    InvalidSettings(String),
    InvalidRequest(String),
    Timeout,
    ConnectionFailed(String),
    Tls(String),
    TooManyRedirects,
    Other(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::InvalidSettings(reason) => {
                write!(formatter, "invalid settings: {reason}")
            }
            TransportError::InvalidRequest(reason) => {
                write!(formatter, "invalid request: {reason}")
            }
            TransportError::Timeout => write!(formatter, "the request timed out"),
            TransportError::ConnectionFailed(reason) => {
                write!(formatter, "connection failed: {reason}")
            }
            TransportError::Tls(reason) => write!(formatter, "TLS error: {reason}"),
            TransportError::TooManyRedirects => write!(formatter, "too many redirects"),
            TransportError::Other(reason) => write!(formatter, "{reason}"),
        }
    }
}

impl Error for TransportError {}

impl TransportError {
    pub(super) fn from_reqwest(error: reqwest::Error) -> Self {
        let error = error.without_url();
        let chain = error_chain(&error);
        if error.is_timeout() {
            TransportError::Timeout
        } else if error.is_redirect() {
            TransportError::TooManyRedirects
        } else if error.is_builder() {
            TransportError::InvalidRequest(chain)
        } else if looks_like_tls(&chain) {
            TransportError::Tls(chain)
        } else if error.is_connect() {
            TransportError::ConnectionFailed(chain)
        } else {
            TransportError::Other(chain)
        }
    }

    pub(super) fn from_io(error: &std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::TimedOut {
            TransportError::Timeout
        } else {
            TransportError::Other(error.to_string())
        }
    }
}

fn error_chain(error: &dyn Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = error.source();
    while let Some(cause) = source {
        parts.push(cause.to_string());
        source = cause.source();
    }
    parts.join(": ")
}

fn looks_like_tls(chain: &str) -> bool {
    let lowered = chain.to_ascii_lowercase();
    TLS_MARKERS.iter().any(|marker| lowered.contains(marker))
}
