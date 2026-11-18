use std::time::Instant;

use requestor_core::event_stream::EventLog;
use requestor_core::response::HttpResponse;
use requestor_core::transport::StreamProgress;

#[derive(Debug)]
pub enum StreamUpdate {
    Opened(Box<HttpResponse>),
    Received(Vec<u8>),
}

pub struct LiveStream {
    response: HttpResponse,
    log: EventLog,
    opened_at: Instant,
}

impl From<StreamProgress<'_>> for StreamUpdate {
    fn from(progress: StreamProgress<'_>) -> Self {
        match progress {
            StreamProgress::Opened(head) => StreamUpdate::Opened(Box::new(head.clone())),
            StreamProgress::Received(bytes) => StreamUpdate::Received(bytes.to_vec()),
        }
    }
}

impl LiveStream {
    pub fn open(head: HttpResponse) -> Self {
        LiveStream {
            response: head,
            log: EventLog::default(),
            opened_at: Instant::now(),
        }
    }

    pub fn receive(&mut self, bytes: &[u8]) {
        self.response.body.extend_from_slice(bytes);
        self.log.receive(bytes);
    }

    pub fn log(&self) -> &EventLog {
        &self.log
    }

    pub fn into_response(self) -> HttpResponse {
        let elapsed = self.response.elapsed + self.opened_at.elapsed();
        HttpResponse {
            elapsed,
            timing: self
                .response
                .timing
                .map(|timing| timing.completed_in(elapsed)),
            ..self.response
        }
    }
}
