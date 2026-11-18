mod support;

use std::net::TcpListener;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

use requestor_core::header::Header;
use requestor_core::method::HttpMethod;
use requestor_core::prepared::PreparedRequest;
use requestor_core::settings::{ProxySettings, RequestSettings};
use requestor_core::transport::{
    SocketCommand, SocketEvent, SocketTransport, TransportError, TungsteniteTransport,
};
use requestor_core::websocket::{Direction, SocketPayload};
use support::{http_response, TestServer};
use tungstenite::handshake::server::{Callback, ErrorResponse, Request, Response};
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::CloseFrame;
use tungstenite::Message;

const WAIT: Duration = Duration::from_secs(5);

fn request(url: String, headers: Vec<Header>) -> PreparedRequest {
    PreparedRequest {
        method: HttpMethod::Get,
        url,
        headers,
        body: None,
        body_source: Default::default(),
    }
}

struct HeaderSpy(Sender<Option<String>>);

impl Callback for HeaderSpy {
    fn on_request(self, request: &Request, response: Response) -> Result<Response, ErrorResponse> {
        let token = request
            .headers()
            .get("x-token")
            .map(|value| value.to_str().unwrap().to_string());
        self.0.send(token).unwrap();
        Ok(response)
    }
}

fn echo_server(seen_header: Sender<Option<String>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}/chat", listener.local_addr().unwrap());
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut socket = tungstenite::accept_hdr(stream, HeaderSpy(seen_header)).unwrap();
        socket.send(Message::binary(vec![1, 2, 3])).unwrap();
        while let Ok(message) = socket.read() {
            match message {
                Message::Text(text) if text.as_str() == "bye" => {
                    let frame = CloseFrame {
                        code: CloseCode::Normal,
                        reason: "done".into(),
                    };
                    socket.close(Some(frame)).unwrap();
                }
                Message::Text(text) => socket.send(Message::text(format!("echo {text}"))).unwrap(),
                _ => {}
            }
        }
    });
    url
}

type Opened = (
    Sender<SocketCommand>,
    Receiver<SocketEvent>,
    Receiver<Result<(), TransportError>>,
);

fn open(request: PreparedRequest) -> Opened {
    open_with(request, RequestSettings::default())
}

fn open_with(
    request: PreparedRequest,
    settings: RequestSettings,
) -> (
    Sender<SocketCommand>,
    Receiver<SocketEvent>,
    Receiver<Result<(), TransportError>>,
) {
    let (commands, command_receiver) = channel();
    let (events, event_receiver) = channel();
    let (done, done_receiver) = channel();
    thread::spawn(move || {
        let transport = TungsteniteTransport::new(&settings);
        let result = transport.open(&request, &command_receiver, &mut |event| {
            events.send(event).ok();
        });
        done.send(result).ok();
    });
    (commands, event_receiver, done_receiver)
}

fn next(events: &Receiver<SocketEvent>) -> SocketEvent {
    events.recv_timeout(WAIT).expect("an event")
}

fn text_of(event: SocketEvent) -> (Direction, String) {
    let SocketEvent::Message(message) = event else {
        panic!("expected a message, got {event:?}");
    };
    (message.direction, message.text().unwrap().to_string())
}

#[test]
fn messages_flow_both_ways_and_headers_reach_the_server() {
    let (seen_sender, seen) = channel();
    let url = echo_server(seen_sender);
    let headers = vec![
        Header::new("X-Token", "secret"),
        Header::new("Host", "ignored"),
    ];
    let (commands, events, done) = open(request(url, headers));
    let SocketEvent::Opened(head) = next(&events) else {
        panic!("expected the handshake");
    };
    assert_eq!(head.status, 101);
    assert_eq!(seen.recv_timeout(WAIT).unwrap().as_deref(), Some("secret"));
    let SocketEvent::Message(binary) = next(&events) else {
        panic!("expected a binary message");
    };
    assert_eq!(binary.payload, SocketPayload::Binary(vec![1, 2, 3]));
    commands
        .send(SocketCommand::Send("hi".to_string()))
        .unwrap();
    assert_eq!(text_of(next(&events)), (Direction::Sent, "hi".to_string()));
    assert_eq!(
        text_of(next(&events)),
        (Direction::Received, "echo hi".to_string())
    );
    commands.send(SocketCommand::Close).unwrap();
    assert_eq!(next(&events), SocketEvent::Closed(None));
    assert_eq!(done.recv_timeout(WAIT).unwrap(), Ok(()));
}

#[test]
fn a_close_from_the_server_reports_its_reason() {
    let (seen_sender, _seen) = channel();
    let url = echo_server(seen_sender);
    let (commands, events, done) = open(request(url, Vec::new()));
    assert!(matches!(next(&events), SocketEvent::Opened(_)));
    assert!(matches!(next(&events), SocketEvent::Message(_)));
    commands
        .send(SocketCommand::Send("bye".to_string()))
        .unwrap();
    assert!(matches!(next(&events), SocketEvent::Message(_)));
    assert_eq!(
        next(&events),
        SocketEvent::Closed(Some("1000 done".to_string()))
    );
    assert_eq!(done.recv_timeout(WAIT).unwrap(), Ok(()));
}

#[test]
fn a_server_that_does_not_upgrade_is_reported() {
    let server = TestServer::spawn(|_| http_response("200 OK", &[], b"plain"));
    let url = server.url("/").replacen("http://", "ws://", 1);
    let (_commands, _events, done) = open(request(url, Vec::new()));
    let error = done.recv_timeout(WAIT).unwrap().unwrap_err();
    assert!(error.to_string().contains("200"), "{error}");
    let handshake = server.requests().join("\n").to_ascii_lowercase();
    assert!(handshake.contains("upgrade: websocket"), "{handshake}");
}

#[test]
fn refused_connections_fail_to_connect() {
    let address = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let (_commands, _events, done) = open(request(format!("ws://{address}/"), Vec::new()));
    let error = done.recv_timeout(WAIT).unwrap().unwrap_err();
    assert!(
        matches!(error, TransportError::ConnectionFailed(_)),
        "{error}"
    );
}

#[test]
fn secure_connections_fail_cleanly_when_the_server_does_not_speak_tls() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("wss://{}/", listener.local_addr().unwrap());
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        drop(stream);
    });
    let (_commands, _events, done) = open(request(url, Vec::new()));
    assert!(done.recv_timeout(WAIT).unwrap().is_err());
}

#[test]
fn dropping_the_commands_closes_the_connection() {
    let (seen_sender, _seen) = channel();
    let url = echo_server(seen_sender);
    let (commands, events, done) = open(request(url, Vec::new()));
    assert!(matches!(next(&events), SocketEvent::Opened(_)));
    drop(commands);
    let closed = std::iter::from_fn(|| events.recv_timeout(WAIT).ok())
        .find(|event| matches!(event, SocketEvent::Closed(_)));
    assert_eq!(closed, Some(SocketEvent::Closed(None)));
    assert_eq!(done.recv_timeout(WAIT).unwrap(), Ok(()));
}

#[test]
fn a_proxy_is_refused_unless_the_host_bypasses_it() {
    let (seen_sender, _seen) = channel();
    let url = echo_server(seen_sender);
    let mut proxy = ProxySettings {
        url: "http://proxy.invalid:8080".to_string(),
        ..ProxySettings::default()
    };
    let settings = RequestSettings {
        proxy: Some(proxy.clone()),
        ..RequestSettings::default()
    };
    let (_commands, _events, done) = open_with(request(url.clone(), Vec::new()), settings);
    let error = done.recv_timeout(WAIT).unwrap().unwrap_err();
    assert!(
        matches!(error, TransportError::InvalidSettings(_)),
        "{error}"
    );
    proxy.set_bypass_hosts_text("127.0.0.1");
    let bypassed = RequestSettings {
        proxy: Some(proxy),
        ..RequestSettings::default()
    };
    let (commands, events, _done) = open_with(request(url, Vec::new()), bypassed);
    assert!(matches!(next(&events), SocketEvent::Opened(_)));
    drop(commands);
}
