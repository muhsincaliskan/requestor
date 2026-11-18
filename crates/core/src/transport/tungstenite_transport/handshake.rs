use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use tungstenite::client::IntoClientRequest;
use tungstenite::handshake::client::{Request, Response};
use tungstenite::http::{HeaderName, HeaderValue, Uri};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::WebSocket;

use super::super::TransportError;
use crate::header::Header;
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;
use crate::settings::RequestSettings;

pub(super) type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

const SECURE_SCHEME: &str = "wss";
const SECURE_PORT: u16 = 443;
const PLAIN_PORT: u16 = 80;
const NO_ADDRESS: &str = "the host name did not resolve to an address";
const HANDSHAKE_OWNED_HEADERS: [&str; 7] = [
    "host",
    "connection",
    "upgrade",
    "sec-websocket-key",
    "sec-websocket-version",
    "sec-websocket-extensions",
    "content-length",
];

pub(super) struct Connection {
    pub(super) socket: Socket,
    pub(super) control: TcpStream,
    pub(super) head: HttpResponse,
}

pub(super) fn connect(
    request: &PreparedRequest,
    settings: &RequestSettings,
) -> Result<Connection, TransportError> {
    let started = Instant::now();
    let handshake = handshake_request(request)?;
    let stream = open_stream(handshake.uri(), settings.connect_timeout())?;
    let control = stream
        .try_clone()
        .map_err(|error| TransportError::from_io(&error))?;
    control
        .set_read_timeout(settings.request_timeout())
        .map_err(|error| TransportError::from_io(&error))?;
    let limit = settings.socket_message_limit();
    let config = WebSocketConfig::default()
        .max_message_size(Some(limit))
        .max_frame_size(Some(limit));
    let (socket, response) =
        tungstenite::client_tls_with_config(handshake, stream, Some(config), None)
            .map_err(TransportError::from_handshake)?;
    Ok(Connection {
        socket,
        control,
        head: handshake_head(&response, started.elapsed()),
    })
}

fn handshake_request(request: &PreparedRequest) -> Result<Request, TransportError> {
    let mut handshake = request
        .url
        .as_str()
        .into_client_request()
        .map_err(TransportError::from_socket)?;
    for header in request
        .headers
        .iter()
        .filter(|header| !is_handshake_owned(header))
    {
        let name = HeaderName::from_bytes(header.name.as_bytes())
            .map_err(|error| TransportError::InvalidRequest(error.to_string()))?;
        let value = HeaderValue::from_str(&header.value)
            .map_err(|error| TransportError::InvalidRequest(error.to_string()))?;
        handshake.headers_mut().append(name, value);
    }
    Ok(handshake)
}

fn is_handshake_owned(header: &Header) -> bool {
    HANDSHAKE_OWNED_HEADERS
        .iter()
        .any(|owned| header.name.eq_ignore_ascii_case(owned))
}

fn open_stream(uri: &Uri, timeout: Option<Duration>) -> Result<TcpStream, TransportError> {
    let host = uri.host().unwrap_or_default();
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let port = uri.port_u16().unwrap_or(default_port(uri));
    let addresses: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|error| TransportError::ConnectionFailed(error.to_string()))?
        .collect();
    let mut last_error = TransportError::ConnectionFailed(NO_ADDRESS.to_string());
    for address in addresses {
        let attempt = match timeout {
            Some(timeout) => TcpStream::connect_timeout(&address, timeout),
            None => TcpStream::connect(address),
        };
        match attempt {
            Ok(stream) => {
                stream.set_nodelay(true).ok();
                return Ok(stream);
            }
            Err(error) => last_error = TransportError::from_connect(&error),
        }
    }
    Err(last_error)
}

fn default_port(uri: &Uri) -> u16 {
    if uri.scheme_str() == Some(SECURE_SCHEME) {
        SECURE_PORT
    } else {
        PLAIN_PORT
    }
}

fn handshake_head(response: &Response, elapsed: Duration) -> HttpResponse {
    let status = response.status();
    HttpResponse {
        status: status.as_u16(),
        reason: status.canonical_reason().unwrap_or_default().to_string(),
        version: format!("{:?}", response.version()),
        headers: response
            .headers()
            .iter()
            .map(|(name, value)| {
                Header::new(name.as_str(), String::from_utf8_lossy(value.as_bytes()))
            })
            .collect(),
        body: Vec::new(),
        truncated: false,
        elapsed,
    }
}
