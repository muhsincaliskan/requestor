use super::super::{BodySource, FormField, PreparedRequest};
use crate::header::Header;
use crate::import::{import_text, Imported};
use crate::method::HttpMethod;

fn request(method: HttpMethod, url: &str) -> PreparedRequest {
    PreparedRequest {
        method,
        url: url.to_string(),
        headers: Vec::new(),
        body: None,
        body_source: BodySource::Inline,
    }
}

#[test]
fn head_requests_use_the_head_flag() {
    let curl = request(HttpMethod::Head, "https://x.test").to_curl();
    assert!(curl.contains("-I"));
    assert!(!curl.contains("-X"));
}

#[test]
fn plain_get_has_no_method_flag() {
    assert_eq!(
        request(HttpMethod::Get, "https://x.test").to_curl(),
        "curl \\\n  'https://x.test'"
    );
}

#[test]
fn secrets_are_masked_by_default_and_included_on_request() {
    let mut prepared = request(HttpMethod::Post, "https://u:pw@x.test/p?api_key=K1&page=2");
    prepared.headers = vec![
        Header::new("Authorization", "Bearer abc"),
        Header::new("Cookie", "sid=1"),
        Header::new("X-Api-Key", "K2"),
        Header::new("Accept", "*/*"),
    ];
    prepared.body = Some(b"client_secret=S&name=ada".to_vec());
    prepared.headers.push(Header::new(
        "Content-Type",
        "application/x-www-form-urlencoded",
    ));
    let masked = prepared.to_curl();
    for leaked in ["abc", "sid=1", "K1", "K2", "pw@", "=S&"] {
        assert!(!masked.contains(leaked), "{leaked} leaked in {masked}");
    }
    assert!(masked.contains("Authorization: <redacted>"));
    assert!(masked.contains("page=2"));
    assert!(masked.contains("Accept: */*"));
    assert!(masked.contains("name=ada"));
    let full = prepared.to_curl_with_secrets();
    for kept in [
        "Bearer abc",
        "sid=1",
        "api_key=K1",
        "u:pw@x.test",
        "client_secret=S",
    ] {
        assert!(full.contains(kept), "{kept} missing in {full}");
    }
}

#[test]
fn file_bodies_export_as_data_binary_from_the_path() {
    let mut prepared = request(HttpMethod::Post, "https://x.test");
    prepared.body = Some(vec![0, 159, 146, 150]);
    prepared.body_source = BodySource::File("/tmp/it's.bin".to_string());
    let curl = prepared.to_curl();
    assert!(curl.contains("--data-binary '@/tmp/it'\\''s.bin'"));
    assert!(!curl.contains("--data-raw"));
}

#[test]
fn multipart_bodies_export_as_form_fields_without_the_boundary_header() {
    let mut prepared = request(HttpMethod::Post, "https://x.test");
    prepared.body = Some(b"--b".to_vec());
    prepared.headers = vec![Header::new(
        "Content-Type",
        "multipart/form-data; boundary=b",
    )];
    prepared.body_source = BodySource::Form(vec![
        FormField::text("title", "@Hi"),
        FormField::text("api_token", "T"),
        FormField::file("doc", "/tmp/a.pdf", Some("application/pdf")),
    ]);
    let curl = prepared.to_curl();
    assert!(!curl.contains("boundary"));
    assert!(curl.contains("--form-string 'title=@Hi'"));
    assert!(curl.contains("--form-string 'api_token=<redacted>'"));
    assert!(curl.contains("-F 'doc=@/tmp/a.pdf;type=application/pdf'"));
    assert!(prepared.to_curl_with_secrets().contains("api_token=T"));
}

#[test]
fn exported_multipart_and_head_commands_import_back() {
    let mut prepared = request(HttpMethod::Post, "https://x.test");
    prepared.body = Some(b"x".to_vec());
    prepared.body_source = BodySource::Form(vec![
        FormField::text("title", "@Hi"),
        FormField::file("doc", "/tmp/a.pdf", Some("application/pdf")),
    ]);
    let Ok(Imported::Request(imported)) = import_text(&prepared.to_curl()) else {
        panic!("expected a request");
    };
    let params = imported.request.body.unwrap().formdata.unwrap();
    assert_eq!(params[0].value.as_deref(), Some("@Hi"));
    assert_eq!(params[1].kind.as_deref(), Some("file"));
    let Ok(Imported::Request(head)) =
        import_text(&request(HttpMethod::Head, "https://x.test").to_curl())
    else {
        panic!("expected a request");
    };
    assert_eq!(head.request.method.as_deref(), Some("HEAD"));
}
