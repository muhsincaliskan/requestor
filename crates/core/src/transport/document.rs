use std::cell::Cell;

use super::{Cancellation, HttpTransport, ReqwestTransport, TransportError};
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;
use crate::settings::RequestSettings;

const DOWNLOAD_TIMEOUT_MS: u64 = 60_000;
const DOWNLOAD_CONNECT_TIMEOUT_MS: u64 = 10_000;
const EVENT_STREAM_REFUSED: &str = "the server answered with an event stream, not a document";

pub struct DocumentTransport(ReqwestTransport);

impl DocumentTransport {
    pub fn new(
        settings: &RequestSettings,
        max_document_bytes: usize,
    ) -> Result<Self, TransportError> {
        ReqwestTransport::new(&settings.for_document_download(max_document_bytes))
            .map(DocumentTransport)
    }
}

impl HttpTransport for DocumentTransport {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        let refuse_streams = Cancellation::default();
        let streamed = Cell::new(false);
        let response = self.0.send_cancellable(request, &refuse_streams, &mut |_| {
            streamed.set(true);
            refuse_streams.cancel();
        });
        if streamed.get() {
            return Err(TransportError::Other(EVENT_STREAM_REFUSED.to_string()));
        }
        response
    }
}

impl RequestSettings {
    pub fn for_document_download(&self, max_document_bytes: usize) -> Self {
        RequestSettings {
            request_timeout_ms: DOWNLOAD_TIMEOUT_MS,
            connect_timeout_ms: DOWNLOAD_CONNECT_TIMEOUT_MS,
            max_response_bytes: max_document_bytes,
            cookies_enabled: false,
            send_no_cache_header: false,
            send_postman_token_header: false,
            ..self.clone()
        }
    }
}
