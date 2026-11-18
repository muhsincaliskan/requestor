use std::time::Duration;

use requestor_core::header::Header;
use requestor_core::response::{HttpResponse, ResponseFormat};

use super::{BodyLayout, ResponseView};

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
        timing: None,
    }
}

#[test]
fn json_is_laid_out_for_display_and_the_raw_body_is_kept() {
    let mut view = ResponseView::from_response(response("application/json", b"{\"a\":1}"), true);
    assert_eq!(view.format, ResponseFormat::Json);
    assert_eq!(view.display().row_count(), 3);
    assert_eq!(view.body_bytes(), b"{\"a\":1}");
    view.set_layout(BodyLayout::Raw);
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

const ITEMS: &[u8] = br#"{"items":[{"id":1,"name":"a"},{"id":2,"name":"b"}],"total":2}"#;

#[test]
fn a_json_filter_replaces_the_shown_body_until_it_is_cleared() {
    let mut view = ResponseView::from_response(response("application/json", ITEMS), true);
    assert!(view.is_inspectable());
    view.set_filter("$.items[*].id".to_string());
    assert_eq!(view.filter_result_count(), Some(2));
    assert_eq!(view.display().text(), "[\n  1,\n  2\n]");
    view.set_layout(BodyLayout::Raw);
    assert_eq!(view.display().text(), "[1,2]");
    view.set_filter(String::new());
    assert_eq!(view.display().text(), std::str::from_utf8(ITEMS).unwrap());
}

#[test]
fn an_invalid_filter_keeps_the_body_and_reports_where_it_failed() {
    let mut view = ResponseView::from_response(response("application/json", ITEMS), true);
    view.set_filter("$.items[".to_string());
    assert_eq!(
        view.inspection().error().map(|error| error.position),
        Some(9)
    );
    assert!(!view.inspection().is_filtering());
    assert_eq!(view.display().row_count(), 13);
}

#[test]
fn the_tree_follows_the_filter_and_is_released_with_the_display() {
    let mut view = ResponseView::from_response(response("application/json", ITEMS), true);
    view.set_layout(BodyLayout::Tree);
    assert_eq!(view.json_tree().map(|root| root.child_count()), Some(2));
    view.set_filter("$..name".to_string());
    assert_eq!(
        view.json_tree().map(|root| root.summary()),
        Some("[2]".to_string())
    );
    view.release_display();
    assert_eq!(view.filter_result_count(), Some(2));
}

#[test]
fn a_body_that_is_not_json_has_no_tree_or_filter_results() {
    let mut view = ResponseView::from_response(response("application/json", b"{oops"), true);
    view.set_filter("$.a".to_string());
    assert_eq!(view.filter_result_count(), None);
    assert!(view.json_tree().is_none());
    assert_eq!(view.display().text(), "{\n  oops");
}

#[test]
fn typing_an_unfinished_filter_keeps_the_last_result() {
    let mut view = ResponseView::from_response(response("application/json", ITEMS), true);
    view.set_filter("$.total".to_string());
    assert_eq!(view.display().text(), "[\n  2\n]");
    view.set_filter("$.total[".to_string());
    assert!(view.inspection().error().is_some());
    assert!(view.holds_display());
    assert_eq!(view.display().text(), "[\n  2\n]");
    view.set_filter(" $.total ".to_string());
    assert!(view.holds_display());
}
