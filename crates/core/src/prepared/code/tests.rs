use super::super::{BodySource, FormField, PreparedRequest};
use super::CodeLanguage;
use crate::environment::Environment;
use crate::header::Header;
use crate::method::HttpMethod;
use crate::secret::Redactor;
use crate::variables::VariableScope;

const JSON_BODY: &str = "{\n  \"name\": \"Rex\"\n}";

pub(super) fn json_post() -> PreparedRequest {
    PreparedRequest {
        method: HttpMethod::Post,
        url: "https://api.test/pets?page=2".to_string(),
        headers: vec![
            Header::new("Content-Type", "application/json"),
            Header::new("Host", "api.test"),
        ],
        body: Some(JSON_BODY.as_bytes().to_vec()),
        body_source: BodySource::Inline,
    }
}

fn form_post() -> PreparedRequest {
    PreparedRequest {
        method: HttpMethod::Post,
        url: "https://api.test/upload".to_string(),
        headers: vec![Header::new(
            "Content-Type",
            "multipart/form-data; boundary=XYZ",
        )],
        body: Some(b"ignored".to_vec()),
        body_source: BodySource::Form(vec![
            FormField::text("title", "Rex"),
            FormField::file("photo", "/tmp/rex.png", Some("image/png")),
        ]),
    }
}

pub(super) fn revealed(request: &PreparedRequest, language: CodeLanguage) -> String {
    request.to_code_with_secrets(language)
}

#[test]
fn curl_matches_the_existing_command() {
    let request = json_post();
    assert_eq!(
        request.to_code(CodeLanguage::Curl, &Redactor::default()),
        request.to_curl(&Redactor::default())
    );
}

#[test]
fn http_file_keeps_the_request_as_written() {
    assert_eq!(
        revealed(&json_post(), CodeLanguage::Http),
        format!("POST https://api.test/pets?page=2\nContent-Type: application/json\nHost: api.test\n\n{JSON_BODY}")
    );
}

#[test]
fn http_file_writes_multipart_parts_with_file_references() {
    let code = revealed(&form_post(), CodeLanguage::Http);
    let expected_body = "--XYZ\nContent-Disposition: form-data; name=\"title\"\n\nRex\n--XYZ\nContent-Disposition: form-data; name=\"photo\"; filename=\"rex.png\"\nContent-Type: image/png\n\n< /tmp/rex.png\n--XYZ--\n";
    assert!(code.ends_with(expected_body), "{code}");
}

#[test]
fn javascript_uses_fetch_with_a_line_per_body_line() {
    let code = revealed(&json_post(), CodeLanguage::JavaScript);
    assert!(code.starts_with(
        "const response = await fetch(\"https://api.test/pets?page=2\", {\n  method: \"POST\",\n"
    ));
    assert!(code.contains("    \"Content-Type\": \"application/json\",\n"));
    assert!(
        code.contains("  body: \"{\\n\" +\n    \"  \\\"name\\\": \\\"Rex\\\"\\n\" +\n    \"}\",\n"),
        "{code}"
    );
    assert!(code.ends_with("console.log(await response.text());\n"));
}

#[test]
fn javascript_forms_use_form_data_and_let_fetch_set_the_boundary() {
    let code = revealed(&form_post(), CodeLanguage::JavaScript);
    assert!(code.starts_with("import { openAsBlob } from \"node:fs\";\n"));
    assert!(code.contains("form.append(\"title\", \"Rex\");\n"));
    assert!(code.contains("form.append(\"photo\", await openAsBlob(\"/tmp/rex.png\", { type: \"image/png\" }), \"rex.png\");\n"));
    assert!(code.contains("  body: form,\n"));
    assert!(!code.contains("boundary"), "{code}");
}

#[test]
fn python_encodes_text_bodies_and_merges_repeated_headers() {
    let mut request = json_post();
    request.headers.push(Header::new("Accept", "text/plain"));
    request
        .headers
        .push(Header::new("accept", "application/json"));
    request.headers.push(Header::new("Cookie", "a=1"));
    request.headers.push(Header::new("Cookie", "b=2"));
    let code = revealed(&request, CodeLanguage::Python);
    assert!(code.starts_with("import requests\n\nurl = \"https://api.test/pets?page=2\"\n"));
    assert!(
        code.contains("    \"Accept\": \"text/plain, application/json\",\n"),
        "{code}"
    );
    assert!(code.contains("    \"Cookie\": \"a=1; b=2\",\n"), "{code}");
    assert!(
        code.contains(
            "data = (\n    \"{\\n\"\n    \"  \\\"name\\\": \\\"Rex\\\"\\n\"\n    \"}\"\n)\n"
        ),
        "{code}"
    );
    assert!(code.contains(
        "response = requests.request(\"POST\", url, headers=headers, data=data.encode())\n"
    ));
}

#[test]
fn python_sends_every_form_field_as_a_multipart_part() {
    let code = revealed(&form_post(), CodeLanguage::Python);
    assert!(code.contains("    (\"title\", (None, \"Rex\")),\n"));
    assert!(code.contains(
        "    (\"photo\", (\"rex.png\", open(\"/tmp/rex.png\", \"rb\"), \"image/png\")),\n"
    ));
    assert!(
        code.contains("requests.request(\"POST\", url, files=files)"),
        "{code}"
    );
}

#[test]
fn java_skips_restricted_headers_and_publishes_the_body() {
    let code = revealed(&json_post(), CodeLanguage::Java);
    assert!(code.starts_with("import java.net.URI;\n"));
    assert!(code.contains(".header(\"Content-Type\", \"application/json\")\n"));
    assert!(!code.contains("\"Host\""), "{code}");
    assert!(
        code.contains(".method(\"POST\", HttpRequest.BodyPublishers.ofString(\n"),
        "{code}"
    );
    assert!(
        code.contains("\"}\"))\n                .build();\n"),
        "{code}"
    );
}

#[test]
fn java_builds_multipart_bodies_from_parts_and_files() {
    let code = revealed(&form_post(), CodeLanguage::Java);
    assert!(code.contains("import java.nio.file.Files;\n"));
    assert!(code.contains("List<byte[]> parts = List.of(\n"));
    assert!(code.contains("Files.readAllBytes(Path.of(\"/tmp/rex.png\"))"));
    assert!(code.contains(".header(\"Content-Type\", \"multipart/form-data; boundary=XYZ\")"));
    assert!(code.contains("(\"\\r\\n\" +\n                        \"--XYZ--\\r\\n\").getBytes(StandardCharsets.UTF_8)"), "{code}");
    assert!(code.contains("BodyPublishers.ofByteArrays(parts))\n"));
}

#[test]
fn bodyless_requests_have_no_body_in_any_language() {
    let request = PreparedRequest::get("https://api.test/", Vec::new());
    assert_eq!(
        revealed(&request, CodeLanguage::Http),
        "GET https://api.test/\n"
    );
    assert!(!revealed(&request, CodeLanguage::JavaScript).contains("body"));
    assert!(!revealed(&request, CodeLanguage::Python).contains("data"));
    assert!(revealed(&request, CodeLanguage::Java).contains("BodyPublishers.noBody())"));
}

#[test]
fn file_bodies_are_read_from_their_path() {
    let mut request = json_post();
    request.body_source = BodySource::File("/tmp/pet.json".to_string());
    assert!(revealed(&request, CodeLanguage::Http).ends_with("\n\n< /tmp/pet.json"));
    assert!(revealed(&request, CodeLanguage::JavaScript)
        .contains("body: await openAsBlob(\"/tmp/pet.json\"),"));
    assert!(
        revealed(&request, CodeLanguage::Python).contains("data = open(\"/tmp/pet.json\", \"rb\")")
    );
    assert!(revealed(&request, CodeLanguage::Java).contains("ofFile(Path.of(\"/tmp/pet.json\"))"));
}

#[test]
fn secrets_are_masked_in_every_language_unless_included() {
    let mut request = json_post();
    request.url = "https://api.test/pets?api_key=K1".to_string();
    request
        .headers
        .push(Header::new("Authorization", "Bearer abc"));
    request.body = Some(b"{\"password\":\"pw1\"}".to_vec());
    let mut form = form_post();
    form.body_source = BodySource::Form(vec![FormField::text("token", "T1")]);
    for language in CodeLanguage::ALL {
        for (prepared, leaked) in [(&request, ["K1", "Bearer abc", "pw1"]), (&form, ["T1"; 3])] {
            let masked = prepared.to_code(language, &Redactor::default());
            for value in leaked {
                assert!(!masked.contains(value), "{value} leaked in {masked}");
            }
        }
        assert!(revealed(&request, language).contains("Bearer abc"));
    }
}

#[test]
fn unknown_stored_languages_fall_back_to_curl() {
    let language: CodeLanguage = serde_json::from_str("\"cobol\"").unwrap();
    assert_eq!(language, CodeLanguage::Curl);
    let stored = serde_json::to_string(&CodeLanguage::JavaScript).unwrap();
    assert_eq!(stored, "\"javascript\"");
}

#[test]
fn an_empty_body_is_no_body() {
    let mut request = PreparedRequest::get("https://api.test/", Vec::new());
    request.body = Some(Vec::new());
    assert!(!revealed(&request, CodeLanguage::JavaScript).contains("body"));
    assert_eq!(
        revealed(&request, CodeLanguage::Http),
        "GET https://api.test/\n"
    );
    assert!(revealed(&request, CodeLanguage::Java).contains("noBody()"));
}

#[test]
fn fetch_leaves_out_a_body_it_cannot_send() {
    let mut request = json_post();
    request.method = HttpMethod::Get;
    let code = revealed(&request, CodeLanguage::JavaScript);
    assert!(code.starts_with("// fetch cannot send a body"), "{code}");
    assert!(!code.contains("body:"), "{code}");
    assert!(revealed(&request, CodeLanguage::Python).contains("data=data.encode()"));
}

#[test]
fn headers_a_library_refuses_are_left_out() {
    let mut request = json_post();
    request.headers.push(Header::new("Expect", "100-continue"));
    request
        .headers
        .push(Header::new("Connection", "keep-alive"));
    for language in [CodeLanguage::JavaScript, CodeLanguage::Java] {
        let code = revealed(&request, language);
        for name in ["Expect", "Connection", "Host"] {
            assert!(!code.contains(&format!("\"{name}\"")), "{name} in {code}");
        }
    }
    assert!(revealed(&request, CodeLanguage::Http).contains("Expect: 100-continue"));
}

#[test]
fn typed_text_parts_keep_their_content_type() {
    let mut request = form_post();
    request.body_source = BodySource::Form(vec![FormField::typed_text(
        "meta",
        "{}",
        Some("application/json"),
    )]);
    let http = revealed(&request, CodeLanguage::Http);
    assert!(
        http.contains("name=\"meta\"\nContent-Type: application/json\n\n{}\n"),
        "{http}"
    );
    let python = revealed(&request, CodeLanguage::Python);
    assert!(
        python.contains("(\"meta\", (None, \"{}\", \"application/json\")),"),
        "{python}"
    );
}

#[test]
fn untyped_file_parts_are_sent_as_octet_stream() {
    let mut request = form_post();
    request.body_source = BodySource::Form(vec![FormField::file("doc", "/tmp/a.bin", None)]);
    let python = revealed(&request, CodeLanguage::Python);
    assert!(python.contains("\"application/octet-stream\""), "{python}");
    assert!(
        revealed(&request, CodeLanguage::Http).contains("Content-Type: application/octet-stream")
    );
}

#[test]
fn multipart_file_paths_are_masked() {
    let mut request = form_post();
    request.body_source =
        BodySource::Form(vec![FormField::file("doc", "/home/ada/s3cret.txt", None)]);
    let environment = Environment::from_json(
        r#"{"name":"E","values":[{"key":"name","value":"s3cret","type":"secret"}]}"#,
    )
    .unwrap();
    let scope = VariableScope::layered(&[], &[], Some(&environment), &[]);
    let redactor = Redactor::for_request(&scope, None);
    for language in CodeLanguage::ALL {
        let code = request.to_code(language, &redactor);
        assert!(!code.contains("s3cret"), "{code}");
    }
}

#[test]
fn large_java_bodies_are_split_into_separate_constants() {
    let mut request = json_post();
    request.body = Some("x".repeat(40_000).into_bytes());
    let code = revealed(&request, CodeLanguage::Java);
    assert!(
        code.contains("ofString(\n                        String.join(\"\",\n"),
        "{}",
        &code[..400]
    );
    assert_eq!(code.matches("\"x").count(), 3);
}
