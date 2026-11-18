mod cancellation;
mod certified;
mod document;
mod error;
mod reading;
mod reqwest_transport;
mod socket;
mod timing;
mod tungstenite_transport;

pub use cancellation::Cancellation;
pub use document::DocumentTransport;
pub use error::TransportError;
pub use reqwest_transport::ReqwestTransport;
pub use socket::{SocketCommand, SocketEvent, SocketTransport};
pub use tungstenite_transport::TungsteniteTransport;

use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;

pub enum StreamProgress<'a> {
    Opened(&'a HttpResponse),
    Received(&'a [u8]),
}

pub trait HttpTransport {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError>;
}
