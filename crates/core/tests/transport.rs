mod support;

use std::time::Duration;

use requestor_core::header::Header;
use requestor_core::method::HttpMethod;
use requestor_core::prepared::PreparedRequest;
use requestor_core::response::ResponseFormat;
use requestor_core::secret::Secret;
use requestor_core::settings::{ProxySettings, RequestSettings};
use requestor_core::transport::{HttpTransport, ReqwestTransport, TransportError};
use support::{http_response, TestServer};

fn request(method: HttpMethod, url: String) -> PreparedRequest {
    PreparedRequest {
        method,
        url,
        headers: Vec::new(),
        body: None,
    }
}

fn transport(settings: &RequestSettings) -> ReqwestTransport {
    ReqwestTransport::new(settings).unwrap()
}

#[test]
fn sends_headers_and_reads_json_response() {
    let server = TestServer::spawn(|_| {
        http_response(
            "200 OK",
            &["Content-Type: application/json", "X-Reply: yes"],
            br#"{"ok":true}"#,
        )
    });
    let mut prepared = request(HttpMethod::Get, server.url("/items?a=1"));
    prepared.headers.push(Header::new("X-Custom", "42"));
    let response = transport(&RequestSettings::default())
        .send(prepared.clone())
        .unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.reason, "OK");
    assert_eq!(response.header("x-reply"), Some("yes"));
    assert_eq!(response.format(), ResponseFormat::Json);
    assert_eq!(response.body_text(), r#"{"ok":true}"#);
    let received = server.requests().remove(0).to_ascii_lowercase();
    assert!(received.starts_with("get /items?a=1 http/1.1"));
    assert!(received.contains("x-custom: 42"));
    assert!(received.contains("user-agent: requestor/"));
}

#[test]
fn sends_request_body_and_custom_method() {
    let server = TestServer::spawn(|_| http_response("201 Created", &[], b"done"));
    let mut prepared = request(
        HttpMethod::Custom("PURGE".to_string()),
        server.url("/cache"),
    );
    prepared.body = Some(b"payload".to_vec());
    let response = transport(&RequestSettings::default())
        .send(prepared.clone())
        .unwrap();
    assert_eq!(response.status, 201);
    let received = server.requests().remove(0);
    assert!(received.starts_with("PURGE /cache"));
    assert!(received.ends_with("payload"));
}

#[test]
fn follows_redirects_only_when_enabled() {
    let server = TestServer::spawn(|request| {
        if request.starts_with("GET /start") {
            http_response("302 Found", &["Location: /final"], b"")
        } else {
            http_response("200 OK", &[], b"arrived")
        }
    });
    let prepared = request(HttpMethod::Get, server.url("/start"));
    let followed = transport(&RequestSettings::default())
        .send(prepared.clone())
        .unwrap();
    assert_eq!(followed.body_text(), "arrived");
    let settings = RequestSettings {
        follow_redirects: false,
        ..RequestSettings::default()
    };
    let stopped = transport(&settings).send(prepared.clone()).unwrap();
    assert_eq!(stopped.status, 302);
    assert_eq!(stopped.header("location"), Some("/final"));
}

#[test]
fn redirect_loops_hit_the_limit() {
    let server = TestServer::spawn(|_| http_response("302 Found", &["Location: /again"], b""));
    let settings = RequestSettings {
        max_redirects: 3,
        ..RequestSettings::default()
    };
    let error = transport(&settings)
        .send(request(HttpMethod::Get, server.url("/again")))
        .unwrap_err();
    assert_eq!(error, TransportError::TooManyRedirects);
}

#[test]
fn caps_response_size_and_reports_truncation() {
    let server = TestServer::spawn(|_| http_response("200 OK", &[], &vec![b'x'; 10_000]));
    let settings = RequestSettings {
        max_response_bytes: 1_000,
        ..RequestSettings::default()
    };
    let response = transport(&settings)
        .send(request(HttpMethod::Get, server.url("/big")))
        .unwrap();
    assert_eq!(response.body.len(), 1_000);
    assert!(response.truncated);
    let whole = transport(&RequestSettings::default())
        .send(request(HttpMethod::Get, server.url("/big")))
        .unwrap();
    assert_eq!(whole.body.len(), 10_000);
    assert!(!whole.truncated);
}

#[test]
fn slow_servers_time_out() {
    let server = TestServer::spawn(|_| {
        std::thread::sleep(Duration::from_millis(1_500));
        http_response("200 OK", &[], b"late")
    });
    let settings = RequestSettings {
        request_timeout_ms: 200,
        ..RequestSettings::default()
    };
    let error = transport(&settings)
        .send(request(HttpMethod::Get, server.url("/slow")))
        .unwrap_err();
    assert_eq!(error, TransportError::Timeout);
}

#[test]
fn refused_connections_are_reported_as_connection_failures() {
    let address = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let error = transport(&RequestSettings::default())
        .send(request(HttpMethod::Get, format!("http://{address}/")))
        .unwrap_err();
    assert!(matches!(error, TransportError::ConnectionFailed(_)));
}

#[test]
fn cookies_are_replayed_only_when_enabled() {
    let server =
        TestServer::spawn(|_| http_response("200 OK", &["Set-Cookie: session=abc; Path=/"], b""));
    for (enabled, expected) in [(true, true), (false, false)] {
        let settings = RequestSettings {
            cookies_enabled: enabled,
            ..RequestSettings::default()
        };
        let client = transport(&settings);
        let prepared = request(HttpMethod::Get, server.url("/"));
        client.send(prepared.clone()).unwrap();
        client.send(prepared.clone()).unwrap();
        let requests = server.requests();
        let last = requests.last().unwrap().to_ascii_lowercase();
        assert_eq!(
            last.contains("cookie: session=abc"),
            expected,
            "enabled={enabled}"
        );
    }
}

#[test]
fn proxy_receives_absolute_requests_with_credentials() {
    let proxy = TestServer::spawn(|_| http_response("200 OK", &[], b"via-proxy"));
    let settings = RequestSettings {
        proxy: Some(ProxySettings {
            url: format!("http://{}", proxy.address),
            username: Some("user".to_string()),
            password: Some(Secret::new("pass")),
            bypass_hosts: Vec::new(),
        }),
        ..RequestSettings::default()
    };
    let response = transport(&settings)
        .send(request(
            HttpMethod::Get,
            "http://origin.invalid/page".to_string(),
        ))
        .unwrap();
    assert_eq!(response.body_text(), "via-proxy");
    let received = proxy.requests().remove(0).to_ascii_lowercase();
    assert!(received.starts_with("get http://origin.invalid/page"));
    assert!(received.contains("proxy-authorization: basic dxnlcjpwyxnz"));
}

#[test]
fn invalid_proxy_url_is_a_settings_error() {
    let settings = RequestSettings {
        proxy: Some(ProxySettings {
            url: "not a url".to_string(),
            username: None,
            password: None,
            bypass_hosts: Vec::new(),
        }),
        ..RequestSettings::default()
    };
    assert!(matches!(
        ReqwestTransport::new(&settings),
        Err(TransportError::InvalidSettings(_))
    ));
}
