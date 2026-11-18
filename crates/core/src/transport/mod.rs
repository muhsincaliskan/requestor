mod cancellation;
mod error;
mod reading;
mod reqwest_transport;

pub use cancellation::Cancellation;
pub use error::TransportError;
pub use reqwest_transport::ReqwestTransport;

use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;

pub enum StreamProgress<'a> {
    Opened(&'a HttpResponse),
    Received(&'a [u8]),
}

pub trait HttpTransport {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError>;
}
