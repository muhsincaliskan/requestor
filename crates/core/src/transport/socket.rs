use std::sync::mpsc::Receiver;

use super::TransportError;
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;
use crate::websocket::SocketMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketCommand {
    Send(String),
    Close,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SocketEvent {
    Opened(HttpResponse),
    Message(SocketMessage),
    Closed(Option<String>),
}

pub trait SocketTransport {
    fn open(
        &self,
        request: &PreparedRequest,
        commands: &Receiver<SocketCommand>,
        on_event: &mut dyn FnMut(SocketEvent),
    ) -> Result<(), TransportError>;
}
