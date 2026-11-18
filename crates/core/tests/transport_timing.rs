use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use requestor_core::method::HttpMethod;
use requestor_core::prepared::PreparedRequest;
use requestor_core::settings::RequestSettings;
use requestor_core::transport::{HttpTransport, ReqwestTransport};

const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const BODY: &str = "ok";
const KEEP_ALIVE_REPLY: &str = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";

#[test]
fn a_fresh_connection_has_dns_and_connect_timings_and_a_reused_one_does_not() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || serve_twice(listener));
    let settings = RequestSettings {
        use_system_proxy: false,
        request_timeout_ms: TEST_TIMEOUT.as_millis() as u64,
        ..RequestSettings::default()
    };
    let transport = ReqwestTransport::new(&settings).unwrap();
    let request = PreparedRequest {
        method: HttpMethod::Get,
        url: format!("http://localhost:{port}/"),
        headers: Vec::new(),
        body: None,
        body_source: Default::default(),
    };

    let first = transport.send(request.clone()).unwrap();
    let fresh = first.timing.unwrap();
    assert_eq!(first.body_text(), BODY);
    assert!(fresh.first_byte >= fresh.dns.unwrap() + fresh.connect.unwrap());
    assert_eq!(first.elapsed, fresh.first_byte + fresh.download);

    let second = transport.send(request).unwrap();
    assert_eq!(second.body_text(), BODY);
    let reused = second.timing.unwrap();
    assert_eq!((reused.dns, reused.connect), (None, None));
    assert_eq!(second.elapsed, reused.first_byte + reused.download);
    server.join().unwrap();
}

fn serve_twice(listener: TcpListener) {
    let (stream, _) = listener.accept().unwrap();
    stream.set_read_timeout(Some(TEST_TIMEOUT)).unwrap();
    let mut stream = BufReader::new(stream);
    for _ in 0..2 {
        for line in stream.by_ref().lines() {
            if line.unwrap().is_empty() {
                break;
            }
        }
        stream
            .get_mut()
            .write_all(KEEP_ALIVE_REPLY.as_bytes())
            .unwrap();
    }
}
