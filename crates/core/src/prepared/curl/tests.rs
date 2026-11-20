use super::super::{BodySource, FormField, PreparedRequest};
use crate::environment::Environment;
use crate::header::Header;
use crate::import::{import_text, Imported};
use crate::method::HttpMethod;
use crate::secret::Redactor;
use crate::variables::VariableScope;

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
    let curl = request(HttpMethod::Head, "https://x.test").to_curl(&Redactor::default());
    assert!(curl.contains("--head"));
    assert!(!curl.contains("--request"));
}

#[test]
fn plain_get_has_no_method_flag() {
    assert_eq!(
        request(HttpMethod::Get, "https://x.test").to_curl(&Redactor::default()),
        "curl --location 'https://x.test'"
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
    let masked = prepared.to_curl(&Redactor::default());
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
    let curl = prepared.to_curl(&Redactor::default());
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
    let curl = prepared.to_curl(&Redactor::default());
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
    let Ok(Imported::Request(imported)) = import_text(&prepared.to_curl(&Redactor::default()))
    else {
        panic!("expected a request");
    };
    let params = imported.request.body.unwrap().formdata.unwrap();
    assert_eq!(params[0].value.as_deref(), Some("@Hi"));
    assert_eq!(params[1].kind.as_deref(), Some("file"));
    let Ok(Imported::Request(head)) =
        import_text(&request(HttpMethod::Head, "https://x.test").to_curl(&Redactor::default()))
    else {
        panic!("expected a request");
    };
    assert_eq!(head.request.method.as_deref(), Some("HEAD"));
}

#[test]
fn json_and_graphql_bodies_are_masked_at_any_depth() {
    let mut prepared = request(HttpMethod::Post, "https://x.test");
    prepared.headers = vec![Header::new("Content-Type", "application/json")];
    prepared.body = Some(
        br#"{"user":"ada","password":"p1","nested":[{"client_secret":"s1"}],"variables":{"apiKey":"k1","id":7}}"#.to_vec(),
    );
    let masked = prepared.to_curl(&Redactor::default());
    for leaked in ["p1", "s1", "k1"] {
        assert!(!masked.contains(leaked), "{leaked} leaked in {masked}");
    }
    assert!(masked.contains("\"user\":\"ada\""));
    assert!(masked.contains("\"id\":7"));
    assert!(prepared
        .to_curl_with_secrets()
        .contains("\"password\":\"p1\""));
}

#[test]
fn unparseable_json_bodies_are_fully_redacted_and_form_types_ignore_case() {
    let mut prepared = request(HttpMethod::Post, "https://x.test");
    prepared.headers = vec![Header::new("Content-Type", "application/json")];
    prepared.body = Some(b"{broken \"password\":\"p1\"".to_vec());
    assert!(!prepared.to_curl(&Redactor::default()).contains("p1"));
    prepared.headers = vec![Header::new(
        "Content-Type",
        "Application/X-WWW-Form-Urlencoded",
    )];
    prepared.body = Some(b"password=p1&a=2".to_vec());
    let masked = prepared.to_curl(&Redactor::default());
    assert!(!masked.contains("p1"));
    assert!(masked.contains("a=2"));
}

#[test]
fn debug_output_of_form_sources_hides_values() {
    let source = BodySource::Form(vec![FormField::text("token", "hunter2")]);
    assert!(!format!("{source:?}").contains("hunter2"));
    assert!(!format!("{:?}", FormField::text("token", "hunter2")).contains("hunter2"));
}

#[test]
fn secret_variable_values_are_masked_wherever_they_appear() {
    let environment = Environment::from_json(
        r#"{"name":"E","values":[{"key":"client","value":"SUPER-SECRET","type":"secret"}]}"#,
    )
    .unwrap();
    let scope = VariableScope::layered(&[], &[], Some(&environment), &[]);
    let redactor = Redactor::for_request(&scope, None);
    let mut prepared = request(HttpMethod::Post, "https://x.test/SUPER-SECRET/items");
    prepared.headers = vec![
        Header::new("X-Client", "SUPER-SECRET"),
        Header::new("Content-Type", "application/json"),
    ];
    prepared.body = Some(br#"{"client":"SUPER-SECRET"}"#.to_vec());
    let masked = prepared.to_curl(&redactor);
    assert!(!masked.contains("SUPER-SECRET"), "{masked}");
    assert!(prepared.to_curl_with_secrets().contains("SUPER-SECRET"));
}

#[test]
fn url_encoded_bodies_list_each_field_like_postman() {
    let mut prepared = request(HttpMethod::Put, "https://x.test");
    prepared.headers = vec![
        Header::new("Content-Type", "application/x-www-form-urlencoded"),
        Header::new("Accept", "*/*"),
    ];
    prepared.body = Some(b"method=login&msisdn=5%2B1&password=p1".to_vec());
    let masked = prepared.to_curl(&Redactor::default());
    assert_eq!(
        masked,
        "curl --location --request PUT 'https://x.test' \\\n\
         --header 'Content-Type: application/x-www-form-urlencoded' \\\n\
         --header 'Accept: */*' \\\n\
         --data-urlencode 'method=login' \\\n\
         --data-urlencode 'msisdn=5+1' \\\n\
         --data-urlencode 'password=<redacted>'"
    );
    assert!(prepared
        .to_curl_with_secrets()
        .contains("--data-urlencode 'password=p1'"));
    let Ok(Imported::Request(imported)) = import_text(&prepared.to_curl_with_secrets()) else {
        panic!("expected a request");
    };
    let fields = imported.request.body.unwrap().urlencoded.unwrap();
    assert_eq!(fields[1].value.as_deref(), Some("5+1"));
}

#[test]
fn form_keys_that_need_encoding_fall_back_to_raw_data() {
    let mut prepared = request(HttpMethod::Post, "https://x.test");
    prepared.headers = vec![Header::new(
        "Content-Type",
        "application/x-www-form-urlencoded",
    )];
    prepared.body = Some(b"a%20b=1".to_vec());
    assert!(prepared
        .to_curl(&Redactor::default())
        .contains("--data-raw 'a%20b=1'"));
    for lossy in ["k=%E9", "k=%zz"] {
        prepared.body = Some(lossy.as_bytes().to_vec());
        let curl = prepared.to_curl_with_secrets();
        assert!(curl.contains(&format!("--data-raw '{lossy}'")), "{curl}");
    }
}
