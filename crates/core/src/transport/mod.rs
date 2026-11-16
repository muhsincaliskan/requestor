mod error;
mod reqwest_transport;

pub use error::TransportError;
pub use reqwest_transport::ReqwestTransport;

use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;

pub trait HttpTransport {
    fn send(&self, request: &PreparedRequest) -> Result<HttpResponse, TransportError>;
}
