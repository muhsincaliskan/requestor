use super::{error_chain, looks_like_tls, TransportError};

const NOT_UPGRADED: &str = "the server did not accept the WebSocket connection, it answered";

impl TransportError {
    pub(in crate::transport) fn from_connect(error: &std::io::Error) -> Self {
        match TransportError::from_io(error) {
            TransportError::Other(reason) if looks_like_tls(&reason) => TransportError::Tls(reason),
            TransportError::Other(reason) => TransportError::ConnectionFailed(reason),
            other => other,
        }
    }

    pub(in crate::transport) fn from_session(error: tungstenite::Error) -> Self {
        match error {
            tungstenite::Error::Io(error) => TransportError::from_io(&error),
            other => TransportError::from_socket(other),
        }
    }

    pub(in crate::transport) fn from_socket(error: tungstenite::Error) -> Self {
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

    pub(in crate::transport) fn from_handshake<S: tungstenite::handshake::HandshakeRole>(
        error: tungstenite::HandshakeError<S>,
    ) -> Self {
        match error {
            tungstenite::HandshakeError::Interrupted(_) => TransportError::Timeout,
            tungstenite::HandshakeError::Failure(error) => TransportError::from_socket(error),
        }
    }
}
