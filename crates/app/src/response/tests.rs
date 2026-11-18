use std::time::Duration;

use requestor_core::header::Header;
use requestor_core::response::{HttpResponse, ResponseFormat};

use super::ResponseView;

fn response(content_type: &str, body: &[u8]) -> HttpResponse {
    HttpResponse {
        status: 200,
        reason: "OK".to_string(),
        version: "HTTP/1.1".to_string(),
        headers: vec![
            Header::new("Content-Type", content_type),
            Header::new("Set-Cookie", "a=1"),
            Header::new("set-cookie", "b=2"),
        ],
        body: body.to_vec(),
        truncated: false,
        elapsed: Duration::from_millis(12),
    }
}

#[test]
fn json_is_laid_out_for_display_and_the_raw_body_is_kept() {
    let mut view = ResponseView::from_response(response("application/json", b"{\"a\":1}"), true);
    assert_eq!(view.format, ResponseFormat::Json);
    assert_eq!(view.display().row_count(), 3);
    assert_eq!(view.body_bytes(), b"{\"a\":1}");
    view.set_pretty(false);
    assert_eq!(view.display().row_count(), 1);
}

#[test]
fn the_display_copy_can_be_released_and_rebuilt() {
    let mut view = ResponseView::from_response(response("text/plain", b"a\nb"), true);
    assert!(!view.holds_display());
    assert_eq!(view.display().row_count(), 2);
    view.release_display();
    assert!(!view.holds_display());
    assert_eq!(view.display().row(1), "b");
}

#[test]
fn format_detection_can_be_turned_off_and_binary_is_not_rendered() {
    let mut plain = ResponseView::from_response(response("application/json", b"{\"a\":1}"), false);
    assert_eq!(plain.format, ResponseFormat::Text);
    assert_eq!(plain.display().row_count(), 1);
    let mut binary =
        ResponseView::from_response(response("image/png", &[0x89, 0x50, 0xff, 0x00]), true);
    assert_eq!(binary.format, ResponseFormat::Binary);
    assert_eq!(binary.display().row(0), "Binary response body");
    assert_eq!(binary.size_bytes(), 4);
}

#[test]
fn cookies_are_read_from_every_set_cookie_header() {
    let view = ResponseView::from_response(response("text/plain", b""), true);
    assert_eq!(view.cookies().collect::<Vec<_>>(), ["a=1", "b=2"]);
}
