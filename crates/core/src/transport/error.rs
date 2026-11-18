use std::error::Error;
use std::fmt;

const TLS_MARKERS: [&str; 2] = ["certificate", "tls"];
const NOT_UPGRADED: &str = "the server did not accept the WebSocket connection, it answered";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    InvalidSettings(String),
    InvalidRequest(String),
    Timeout,
    Cancelled,
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
            TransportError::Cancelled => write!(formatter, "the request was cancelled"),
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
        let wrapped_timeout = error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<reqwest::Error>())
            .is_some_and(reqwest::Error::is_timeout);
        if wrapped_timeout || error.kind() == std::io::ErrorKind::TimedOut {
            TransportError::Timeout
        } else {
            TransportError::Other(error.to_string())
        }
    }
}

impl TransportError {
    pub(super) fn from_connect(error: &std::io::Error) -> Self {
        match TransportError::from_io(error) {
            TransportError::Other(reason) if looks_like_tls(&reason) => TransportError::Tls(reason),
            TransportError::Other(reason) => TransportError::ConnectionFailed(reason),
            other => other,
        }
    }

    pub(super) fn from_socket(error: tungstenite::Error) -> Self {
        match error {
            tungstenite::Error::Io(error) => TransportError::from_connect(&error),
            tungstenite::Error::Tls(error) => TransportError::Tls(error_chain(&error)),
            tungstenite::Error::Url(error) => TransportError::InvalidRequest(error.to_string()),
            tungstenite::Error::HttpFormat(error) => {
                TransportError::InvalidRequest(error.to_string())
            }
            tungstenite::Error::Http(response) => {
                TransportError::Other(format!("{NOT_UPGRADED} {}", response.status()))
            }
            other => TransportError::Other(other.to_string()),
        }
    }

    pub(super) fn from_handshake<S: tungstenite::handshake::HandshakeRole>(
        error: tungstenite::HandshakeError<S>,
    ) -> Self {
        match error {
            tungstenite::HandshakeError::Interrupted(_) => TransportError::Timeout,
            tungstenite::HandshakeError::Failure(error) => TransportError::from_socket(error),
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
