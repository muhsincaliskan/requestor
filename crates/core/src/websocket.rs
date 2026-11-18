mod log;

pub use log::{SocketLog, MAX_KEPT_MESSAGES, MAX_KEPT_MESSAGE_BYTES};

use std::time::Duration;

use crate::draft::RequestDraft;
use crate::prepared::PreparedRequest;

const SCHEMES: [&str; 2] = ["ws://", "wss://"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketPayload {
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketMessage {
    pub direction: Direction,
    pub payload: SocketPayload,
    pub at: Duration,
}

impl SocketMessage {
    pub fn sent(text: impl Into<String>, at: Duration) -> Self {
        SocketMessage {
            direction: Direction::Sent,
            payload: SocketPayload::Text(text.into()),
            at,
        }
    }

    pub fn received(payload: SocketPayload, at: Duration) -> Self {
        SocketMessage {
            direction: Direction::Received,
            payload,
            at,
        }
    }

    pub fn size(&self) -> usize {
        match &self.payload {
            SocketPayload::Text(text) => text.len(),
            SocketPayload::Binary(bytes) => bytes.len(),
        }
    }

    pub fn text(&self) -> Option<&str> {
        match &self.payload {
            SocketPayload::Text(text) => Some(text),
            SocketPayload::Binary(_) => None,
        }
    }
}

impl RequestDraft {
    pub fn is_websocket(&self) -> bool {
        is_websocket_url(&self.url)
    }
}

impl PreparedRequest {
    pub fn is_websocket(&self) -> bool {
        is_websocket_url(&self.url)
    }
}

pub fn is_websocket_url(url: &str) -> bool {
    let url = url.trim_start();
    SCHEMES.iter().any(|scheme| {
        url.get(..scheme.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(scheme))
    })
}

#[cfg(test)]
mod tests;
