use crate::collection::BodyMode;
use crate::collection::Url;
use crate::import::{ImportError, ImportedRequest};

fn import(command: &str) -> ImportedRequest {
    ImportedRequest::from_curl(command).unwrap()
}

#[test]
fn simple_get_defaults_to_http_scheme() {
    let imported = import("curl example.com/items?a=1");
    assert_eq!(imported.request.method.as_deref(), Some("GET"));
    assert_eq!(
        imported.request.url.unwrap().to_text(),
        "http://example.com/items?a=1"
    );
    assert_eq!(imported.name, "GET example.com/items");
}

#[test]
fn json_post_with_headers_and_basic_auth() {
    let command = "curl -X POST 'https://api.test/v1/users' \\\n  -H 'Content-Type: application/json' \\\n  -H \"Accept: */*\" \\\n  -u ada:s3cret \\\n  -d '{\"name\":\"Ada\"}'";
    let imported = import(command);
    let request = imported.request;
    assert_eq!(request.method.as_deref(), Some("POST"));
    assert_eq!(request.header.as_ref().unwrap().len(), 2);
    let body = request.body.unwrap();
    assert_eq!(body.mode, Some(BodyMode::Raw));
    assert_eq!(body.raw.as_deref(), Some("{\"name\":\"Ada\"}"));
    assert_eq!(request.auth.unwrap().value_of("username"), Some("ada"));
}

#[test]
fn data_without_method_posts_and_form_pairs_become_urlencoded() {
    let imported = import("curl https://x.test -d 'a=1&b=hello%20world' -d c=3");
    assert_eq!(imported.request.method.as_deref(), Some("POST"));
    let params = imported.request.body.unwrap().urlencoded.unwrap();
    assert_eq!(params.len(), 3);
    assert_eq!(params[1].value.as_deref(), Some("hello world"));
}

#[test]
fn multipart_forms_distinguish_text_and_files() {
    let imported =
        import("curl -F title=Hi -F 'doc=@/tmp/a.pdf;type=application/pdf' https://x.test/up");
    let params = imported.request.body.unwrap().formdata.unwrap();
    assert_eq!(params[0].value.as_deref(), Some("Hi"));
    assert_eq!(params[1].kind.as_deref(), Some("file"));
}

#[test]
fn get_flag_moves_data_to_the_query_and_head_sets_the_method() {
    let imported = import("curl -G https://x.test/s -d q=rust -d page=2");
    assert_eq!(
        imported.request.url.unwrap().to_text(),
        "https://x.test/s?q=rust&page=2"
    );
    assert_eq!(imported.request.method.as_deref(), Some("GET"));
    assert_eq!(
        import("curl -I https://x.test").request.method.as_deref(),
        Some("HEAD")
    );
}

#[test]
fn clustered_short_flags_and_attached_values() {
    let imported = import("curl -sSLXPUT -H'X-A: 1' https://x.test");
    assert_eq!(imported.request.method.as_deref(), Some("PUT"));
    assert_eq!(imported.request.header_value("x-a"), Some("1"));
}

#[test]
fn json_option_sets_headers_and_body() {
    let imported = import("curl --json '{\"a\":1}' https://x.test");
    assert_eq!(
        imported.request.header_value("content-type"),
        Some("application/json")
    );
    assert_eq!(imported.request.method.as_deref(), Some("POST"));
}

#[test]
fn missing_url_is_an_error() {
    assert_eq!(
        ImportedRequest::from_curl("curl -X POST").unwrap_err(),
        ImportError::MissingUrl
    );
}

fn warnings(imported: &ImportedRequest) -> String {
    imported.warnings.join("\n")
}

#[test]
fn unsupported_options_consume_their_values_and_keep_the_url() {
    let imported = import(
        "curl -k --retry 3 --limit-rate 1k --max-redirs 5 --resolve a:443:1.2.3.4 --oauth2-bearer tok --url-query a=b -D headers.txt -o out.bin -x http://proxy:8080 -m 10 https://x.test/path",
    );
    let text = warnings(&imported);
    assert_eq!(
        imported.request.url.unwrap().to_text(),
        "https://x.test/path"
    );
    assert!(text.contains("--insecure"));
    assert!(text.contains("--limit-rate"));
    assert!(text.contains("--output"));
    assert!(text.contains("--dump-header"));
    assert!(text.contains("--proxy"));
    assert!(text.contains("--max-time"));
    assert!(!text.contains("other"));
}

#[test]
fn unmapped_short_flags_are_named_in_warnings() {
    let imported = import("curl -4 https://x.test");
    assert!(warnings(&imported).contains("-4"));
}

#[test]
fn data_urlencode_is_stored_decoded_and_not_double_encoded() {
    let imported = import("curl https://x.test --data-urlencode 'q=a b&c' --data-urlencode 'plain text' --data-urlencode '=lead'");
    let params = imported.request.body.unwrap().urlencoded.unwrap();
    assert_eq!(params[0].key.as_deref(), Some("q"));
    assert_eq!(params[0].value.as_deref(), Some("a b&c"));
    assert_eq!(params[1].key.as_deref(), Some(""));
    assert_eq!(params[1].value.as_deref(), Some("plain text"));
    assert_eq!(params[2].value.as_deref(), Some("lead"));
}

#[test]
fn get_flag_puts_data_urlencode_into_the_encoded_query() {
    let imported = import(
        "curl -G https://x.test/s -d page=2 --data-urlencode 'q=a b' --data-urlencode 'raw&text'",
    );
    assert_eq!(
        imported.request.url.unwrap().to_text(),
        "https://x.test/s?page=2&q=a%20b&raw%26text"
    );
    assert!(imported.request.body.is_none());
    assert_eq!(imported.request.method.as_deref(), Some("GET"));
}

#[test]
fn data_from_a_file_becomes_a_file_body() {
    for command in [
        "curl https://x.test -d @/tmp/a.json",
        "curl https://x.test --data-binary @/tmp/a.json",
    ] {
        let imported = import(command);
        assert_eq!(imported.request.method.as_deref(), Some("POST"));
        let body = imported.request.body.unwrap();
        assert_eq!(body.file_source(), Some("/tmp/a.json"));
        assert!(imported.warnings.is_empty());
    }
}

#[test]
fn data_raw_keeps_the_at_sign_literal() {
    let imported = import("curl https://x.test --data-raw @literal");
    assert_eq!(
        imported.request.body.unwrap().raw.as_deref(),
        Some("@literal")
    );
}

#[test]
fn unsupported_file_inputs_are_reported() {
    let stdin = import("curl https://x.test -d @-");
    assert!(warnings(&stdin).contains("standard input"));
    assert!(stdin.request.body.is_none());
    let urlencoded = import("curl https://x.test --data-urlencode name@/tmp/v.txt");
    assert!(warnings(&urlencoded).contains("/tmp/v.txt"));
    assert!(urlencoded.request.body.is_none());
    let mixed = import("curl https://x.test -d a=1 -d @/tmp/b");
    assert!(warnings(&mixed).contains("ignored"));
    assert_eq!(mixed.request.body.unwrap().file_source(), Some("/tmp/b"));
}

#[test]
fn form_file_keeps_its_content_type_and_form_string_stays_text() {
    let imported = import("curl -F 'doc=@/tmp/a.pdf;type=application/pdf' --form-string 'note=@not-a-file' https://x.test");
    let params = imported.request.body.unwrap().formdata.unwrap();
    assert_eq!(params[0].content_type.as_deref(), Some("application/pdf"));
    assert_eq!(params[1].kind, None);
    assert_eq!(params[1].value.as_deref(), Some("@not-a-file"));
}

#[test]
fn repeated_json_options_concatenate_without_separator() {
    let imported = import("curl --json '{\"a\":' --json '1}' https://x.test");
    assert_eq!(
        imported.request.body.unwrap().raw.as_deref(),
        Some("{\"a\":1}")
    );
}

#[test]
fn json_option_from_a_file_becomes_a_file_body() {
    let imported = import("curl --json @/tmp/a.json https://x.test");
    assert_eq!(
        imported.request.header_value("accept"),
        Some("application/json")
    );
    assert_eq!(
        imported.request.body.unwrap().file_source(),
        Some("/tmp/a.json")
    );
}

#[test]
fn content_type_selects_the_raw_body_language() {
    let imported = import("curl https://x.test -H 'Content-Type: text/html' -d '<p>'");
    let body = imported.request.body.unwrap();
    assert_eq!(body.raw_content_type(), "text/html");
}

#[test]
fn explicit_method_and_head_flag_resolve_in_order() {
    assert_eq!(
        import("curl -I -X OPTIONS https://x.test")
            .request
            .method
            .as_deref(),
        Some("OPTIONS")
    );
    assert_eq!(
        import("curl -d a=1 -G https://x.test")
            .request
            .method
            .as_deref(),
        Some("GET")
    );
}

#[test]
fn url_text_keeps_a_bare_host_scheme_default() {
    let imported = import("curl localhost:8080/x");
    assert!(matches!(imported.request.url, Some(Url::Parts(_))));
    assert_eq!(
        imported.request.url.unwrap().to_text(),
        "http://localhost:8080/x"
    );
}
