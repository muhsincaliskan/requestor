mod handshake;
mod session;

use std::sync::mpsc::Receiver;

use super::socket::{SocketCommand, SocketEvent, SocketTransport};
use super::TransportError;
use crate::prepared::PreparedRequest;
use crate::settings::RequestSettings;
use session::Session;

pub struct TungsteniteTransport {
    settings: RequestSettings,
}

impl TungsteniteTransport {
    pub fn new(settings: &RequestSettings) -> Self {
        TungsteniteTransport {
            settings: settings.clone(),
        }
    }
}

impl SocketTransport for TungsteniteTransport {
    fn open(
        &self,
        request: &PreparedRequest,
        commands: &Receiver<SocketCommand>,
        on_event: &mut dyn FnMut(SocketEvent),
    ) -> Result<(), TransportError> {
        let connection = handshake::connect(request, &self.settings)?;
        on_event(SocketEvent::Opened(connection.head.clone()));
        Session::new(connection)?.run(commands, on_event)
    }
}
