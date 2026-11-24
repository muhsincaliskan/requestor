use std::io::ErrorKind;
use std::net::Shutdown;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use tungstenite::error::ProtocolError;
use tungstenite::protocol::CloseFrame;
use tungstenite::{Error, Message};

use super::super::socket::{SocketCommand, SocketEvent};
use super::super::TransportError;
use super::handshake::{tcp_stream, Connection, Socket};
use crate::websocket::{SocketMessage, SocketPayload};

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const CLOSE_GRACE: Duration = Duration::from_secs(2);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const NO_STREAM: &str = "the connection has no TCP stream to configure";

enum Flow {
    Continue,
    Finished,
}

pub(super) struct Session {
    socket: Socket,
    opened: Instant,
    close_reason: Option<String>,
}

impl Session {
    pub(super) fn new(connection: Connection) -> Result<Self, TransportError> {
        let stream = tcp_stream(&connection.socket)
            .ok_or_else(|| TransportError::ConnectionFailed(NO_STREAM.to_string()))?;
        stream
            .set_read_timeout(Some(POLL_INTERVAL))
            .and_then(|()| stream.set_write_timeout(Some(WRITE_TIMEOUT)))
            .map_err(|error| TransportError::from_io(&error))?;
        Ok(Session {
            socket: connection.socket,
            opened: Instant::now(),
            close_reason: None,
        })
    }

    pub(super) fn run(
        mut self,
        commands: &Receiver<SocketCommand>,
        on_event: &mut dyn FnMut(SocketEvent),
    ) -> Result<(), TransportError> {
        loop {
            let flow = match commands.try_recv() {
                Ok(SocketCommand::Send(text)) => self.send(text, on_event)?,
                Ok(SocketCommand::Close) | Err(TryRecvError::Disconnected) => self.close(),
                Err(TryRecvError::Empty) => self.receive(on_event)?,
            };
            if let Flow::Finished = flow {
                on_event(SocketEvent::Closed(self.close_reason.take()));
                return Ok(());
            }
        }
    }

    fn send(
        &mut self,
        text: String,
        on_event: &mut dyn FnMut(SocketEvent),
    ) -> Result<Flow, TransportError> {
        if let Err(error) = self.socket.send(Message::text(text.as_str())) {
            return after_send_error(error);
        }
        let message = SocketMessage::sent(text, self.opened.elapsed());
        on_event(SocketEvent::Message(message));
        Ok(Flow::Continue)
    }

    fn receive(&mut self, on_event: &mut dyn FnMut(SocketEvent)) -> Result<Flow, TransportError> {
        let payload = match self.socket.read() {
            Ok(Message::Text(text)) => SocketPayload::Text(text.to_string()),
            Ok(Message::Binary(bytes)) => SocketPayload::Binary(bytes.to_vec()),
            Ok(Message::Close(frame)) => {
                self.close_reason = frame.map(close_reason);
                return Ok(Flow::Continue);
            }
            Ok(_) => return Ok(Flow::Continue),
            Err(error) => return after_read_error(error),
        };
        let message = SocketMessage::received(payload, self.opened.elapsed());
        on_event(SocketEvent::Message(message));
        Ok(Flow::Continue)
    }

    fn close(&mut self) -> Flow {
        if self.socket.close(None).is_err() {
            return Flow::Finished;
        }
        let deadline = Instant::now() + CLOSE_GRACE;
        while Instant::now() < deadline {
            match self.socket.read() {
                Ok(_) => {}
                Err(Error::Io(error)) if is_idle(error.kind()) => {}
                Err(_) => break,
            }
        }
        if let Some(stream) = tcp_stream(&self.socket) {
            stream.shutdown(Shutdown::Both).ok();
        }
        Flow::Finished
    }
}

fn after_read_error(error: Error) -> Result<Flow, TransportError> {
    match error {
        Error::ConnectionClosed | Error::AlreadyClosed => Ok(Flow::Finished),
        Error::Io(error) if is_idle(error.kind()) => Ok(Flow::Continue),
        other => Err(TransportError::from_session(other)),
    }
}

fn after_send_error(error: Error) -> Result<Flow, TransportError> {
    match error {
        Error::ConnectionClosed
        | Error::AlreadyClosed
        | Error::Protocol(ProtocolError::SendAfterClosing) => Ok(Flow::Finished),
        other => Err(TransportError::from_session(other)),
    }
}

fn is_idle(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

fn close_reason(frame: CloseFrame) -> String {
    let code = u16::from(frame.code);
    if frame.reason.is_empty() {
        code.to_string()
    } else {
        format!("{code} {}", frame.reason)
    }
}
