mod log;
mod parser;

pub use log::{EventLog, MAX_KEPT_EVENTS};
pub use parser::EventStreamParser;

use crate::header::CONTENT_TYPE;
use crate::media_type;
use crate::response::HttpResponse;

pub const EVENT_STREAM: &str = "text/event-stream";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServerEvent {
    pub name: Option<String>,
    pub data: String,
    pub id: Option<String>,
}

impl ServerEvent {
    pub fn parse_all(body: &[u8]) -> Vec<ServerEvent> {
        let mut parser = EventStreamParser::default();
        let mut events = parser.push(body);
        events.extend(parser.finish());
        events
    }

    pub fn kind(&self) -> &str {
        self.name.as_deref().unwrap_or(DEFAULT_EVENT_NAME)
    }
}

impl HttpResponse {
    pub fn is_event_stream(&self) -> bool {
        self.header(CONTENT_TYPE).is_some_and(is_event_stream)
    }
}

pub fn is_event_stream(content_type: &str) -> bool {
    media_type::base(content_type).eq_ignore_ascii_case(EVENT_STREAM)
}

const DEFAULT_EVENT_NAME: &str = "message";

#[cfg(test)]
mod tests;
