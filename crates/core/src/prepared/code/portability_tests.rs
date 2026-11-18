use super::super::PreparedRequest;
use super::tests::{json_post, revealed};
use super::CodeLanguage;
use crate::header::Header;
use crate::variables::VariableScope;

#[test]
fn code_can_be_written_for_files_that_are_not_on_this_computer() {
    use crate::collection::{Body, Param, Request, Url};
    use crate::settings::RequestSettings;
    let mut request = Request::new("POST", Url::Raw("https://api.test/upload".to_string()));
    request.body = Some(Body::formdata(vec![Param::file(
        "doc",
        "/missing/report.pdf",
    )]));
    let scope = VariableScope::new();
    let settings = RequestSettings::default();
    assert!(PreparedRequest::from_request(&request, &scope, &settings).is_err());
    let prepared = PreparedRequest::referencing_files(&request, &scope, &settings).unwrap();
    let code = prepared.to_code_with_secrets(CodeLanguage::Python);
    assert!(
        code.contains("open(\"/missing/report.pdf\", \"rb\")"),
        "{code}"
    );
    request.body = Some(Body::file("/missing/body.bin"));
    let prepared = PreparedRequest::referencing_files(&request, &scope, &settings).unwrap();
    assert!(prepared
        .to_code_with_secrets(CodeLanguage::Http)
        .ends_with("< /missing/body.bin"));
}

#[test]
fn non_ascii_header_values_are_sent_as_utf8_bytes() {
    let mut request = json_post();
    request.headers.push(Header::new("X-Name", "Çağrı"));
    let javascript = revealed(&request, CodeLanguage::JavaScript);
    assert!(
        javascript
            .contains("\"X-Name\": String.fromCharCode(...new TextEncoder().encode(\"Çağrı\")),"),
        "{javascript}"
    );
    let python = revealed(&request, CodeLanguage::Python);
    assert!(
        python.contains("\"X-Name\": \"Çağrı\".encode(),"),
        "{python}"
    );
    let java = revealed(&request, CodeLanguage::Java);
    assert!(
        java.contains("// HttpClient accepts only ASCII header values"),
        "{java}"
    );
    assert!(!revealed(&json_post(), CodeLanguage::Java).contains("// HttpClient"));
}
