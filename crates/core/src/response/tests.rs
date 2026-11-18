use std::time::Duration;

use super::{HttpResponse, ResponseFormat, StatusClass};

fn with_status(status: u16) -> HttpResponse {
    HttpResponse {
        status,
        reason: String::new(),
        version: String::new(),
        headers: Vec::new(),
        body: Vec::new(),
        truncated: false,
        elapsed: Duration::ZERO,
        timing: None,
    }
}

#[test]
fn status_classes_are_distinguished() {
    let expectations = [
        (200, true, false),
        (204, true, false),
        (302, false, false),
        (399, false, false),
        (400, false, true),
        (503, false, true),
    ];
    for (status, success, error) in expectations {
        let response = with_status(status);
        assert_eq!(response.is_success(), success, "{status}");
        assert_eq!(response.is_client_or_server_error(), error, "{status}");
    }
}

#[test]
fn status_codes_map_to_their_class() {
    let expectations = [
        (99, StatusClass::Unknown),
        (101, StatusClass::Informational),
        (204, StatusClass::Success),
        (301, StatusClass::Redirect),
        (404, StatusClass::ClientError),
        (500, StatusClass::ServerError),
        (600, StatusClass::Unknown),
    ];
    for (status, class) in expectations {
        assert_eq!(with_status(status).status_class(), class, "{status}");
    }
    assert!(StatusClass::ServerError.is_error());
    assert!(!StatusClass::Redirect.is_error());
}

#[test]
fn declared_content_type_wins() {
    assert_eq!(
        ResponseFormat::detect(Some("application/problem+json"), b"x"),
        ResponseFormat::Json
    );
    assert_eq!(
        ResponseFormat::detect(Some("text/html; charset=utf-8"), b"{}"),
        ResponseFormat::Html
    );
    assert_eq!(
        ResponseFormat::detect(Some("image/png"), b"abc"),
        ResponseFormat::Binary
    );
}

#[test]
fn json_and_xml_have_a_pretty_layout() {
    assert!(ResponseFormat::Xml.has_layout());
    assert_eq!(
        ResponseFormat::Xml.laid_out("<a><b/></a>"),
        "<a>\n  <b/>\n</a>"
    );
    assert!(!ResponseFormat::Html.has_layout());
    assert_eq!(ResponseFormat::Text.laid_out("<a><b/></a>"), "<a><b/></a>");
}

#[test]
fn truncated_utf8_tail_is_still_text() {
    let mut body = "héllo".as_bytes().to_vec();
    body.pop();
    assert_eq!(ResponseFormat::detect(None, &body), ResponseFormat::Text);
}

#[test]
fn sniffs_body_when_type_is_missing_or_generic() {
    assert_eq!(
        ResponseFormat::detect(None, b"  {\"a\":1}"),
        ResponseFormat::Json
    );
    assert_eq!(
        ResponseFormat::detect(Some("application/x-unknown"), b"<a/>"),
        ResponseFormat::Xml
    );
    assert_eq!(
        ResponseFormat::detect(None, &[0xff, 0xfe, 0x00]),
        ResponseFormat::Binary
    );
    assert_eq!(ResponseFormat::detect(None, b"hello"), ResponseFormat::Text);
}
