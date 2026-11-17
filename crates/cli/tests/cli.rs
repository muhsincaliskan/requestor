mod support;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use support::{http_response, TestServer};

const REDACTED: &str = "<redacted>";
const SECRET_TOKEN: &str = "s3cr3t-value";

fn scratch(name: &str, content: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("requestor-cli-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join(name);
    fs::write(&path, content).unwrap();
    path
}

fn requestor(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_requestor"))
        .args(arguments)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn request_path(request: &str) -> Option<&str> {
    let target = request.split_whitespace().nth(1)?;
    target.split('?').next()
}

fn router() -> TestServer {
    TestServer::spawn(|request| match request_path(request) {
        Some("/ok") => http_response("200 OK", &["Content-Type: text/plain"], b"hello"),
        Some("/missing") => http_response("404 Not Found", &[], b"nope"),
        Some("/moved") => http_response("302 Found", &["Location: /ok"], b""),
        Some("/cookie") => http_response("200 OK", &["Set-Cookie: session=abc"], b"x"),
        _ => http_response("500 Internal Server Error", &[], b""),
    })
}

fn collection(server: &TestServer) -> PathBuf {
    let document = format!(
        r#"{{
            "info": {{"name": "demo"}},
            "variable": [{{"key": "base", "value": "http://{}"}}],
            "item": [
                {{"name": "Good", "request": {{"method": "GET", "url": "{{{{base}}}}/ok"}}}},
                {{"name": "Group", "item": [
                    {{"name": "Bad", "request": {{"method": "GET", "url": "{{{{base}}}}/missing"}}}},
                    {{"name": "Redirect", "request": "{{{{base}}}}/moved"}}
                ]}}
            ]
        }}"#,
        server.address
    );
    scratch("collection.json", &document)
}

#[test]
fn run_reports_each_request_and_fails_when_any_request_fails() {
    let server = router();
    let path = collection(&server);
    let output = requestor(&["run", path.to_str().unwrap(), "--no-follow-redirects"]);
    assert_eq!(output.status.code(), Some(1));
    let lines: Vec<String> = stdout(&output).lines().map(str::to_string).collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("Good: GET http://"));
    assert!(lines[0].contains("-> 200") && lines[0].ends_with(" B"));
    assert!(lines[1].contains("Group/Bad") && lines[1].contains("-> 404"));
    assert!(lines[2].contains("Group/Redirect") && lines[2].contains("-> 302"));
}

#[test]
fn run_succeeds_when_selection_only_has_good_requests() {
    let server = router();
    let path = collection(&server);
    let output = requestor(&["run", path.to_str().unwrap(), "--request", "Good"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output).lines().count(), 1);
}

#[test]
fn run_folder_selection_and_json_output() {
    let server = router();
    let path = collection(&server);
    let output = requestor(&["run", path.to_str().unwrap(), "--folder", "Group", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let lines: Vec<String> = stdout(&output).lines().map(str::to_string).collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with(r#"{"name":"Group/Bad","method":"GET","url":"http://"#));
    assert!(lines[0].contains(r#""ok":false,"status":404"#));
    assert!(lines[1].contains(r#""ok":true,"status":200"#));
}

#[test]
fn run_applies_variable_overrides_and_environment() {
    let server = router();
    let environment = scratch(
        "env.json",
        &format!(
            r#"{{"name":"E","values":[{{"key":"base","value":"http://{}"}},{{"key":"path","value":"/missing"}}]}}"#,
            server.address
        ),
    );
    let document = scratch(
        "vars.json",
        r#"{"info":{"name":"v"},"item":[{"name":"One","request":"{{base}}{{path}}"}]}"#,
    );
    let (document, environment) = (document.to_str().unwrap(), environment.to_str().unwrap());
    let overridden = requestor(&["run", document, "--env", environment, "--var", "path=/ok"]);
    assert_eq!(overridden.status.code(), Some(0), "{}", stderr(&overridden));
    let plain = requestor(&["run", document, "--env", environment]);
    assert_eq!(plain.status.code(), Some(1));
}

#[test]
fn run_never_prints_secret_query_values() {
    let server = router();
    let document = scratch(
        "secret.json",
        &format!(
            r#"{{"info":{{"name":"s"}},"item":[{{"name":"One","request":"http://{}/ok?api_key={SECRET_TOKEN}"}}]}}"#,
            server.address
        ),
    );
    let output = requestor(&["run", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains(&format!("api_key={REDACTED}")));
    assert!(!stdout(&output).contains(SECRET_TOKEN));
    assert!(!stderr(&output).contains(SECRET_TOKEN));
}

#[test]
fn run_counts_connection_failures_as_failed_requests() {
    let document = scratch(
        "refused.json",
        r#"{"info":{"name":"r"},"item":[{"name":"Down","request":"http://127.0.0.1:1/x"}]}"#,
    );
    let output = requestor(&["run", document.to_str().unwrap(), "--timeout-ms", "2000"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("FAILED"));
}

#[test]
fn usage_errors_exit_with_two() {
    assert_eq!(requestor(&[]).status.code(), Some(2));
    assert_eq!(requestor(&["run"]).status.code(), Some(2));
    assert_eq!(
        requestor(&["run", "missing-file.json"]).status.code(),
        Some(2)
    );
    let output = requestor(&["run", "x.json", "--bogus"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unknown option"));
}

#[test]
fn send_prints_status_headers_and_body_from_curl_text() {
    let server = router();
    let command = format!("curl {}", server.url("/ok"));
    let output = requestor(&["send", &command]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.starts_with("HTTP/1.1 200 OK\n"));
    assert!(text.contains("content-type: text/plain"));
    assert!(text.ends_with("\n\nhello"));
}

#[test]
fn send_accepts_separate_curl_arguments_and_masks_secret_headers() {
    let server = router();
    let url = server.url("/cookie");
    let output = requestor(&["send", "curl", "-X", "POST", "-d", "a=1", &url]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).contains(&format!("set-cookie: {REDACTED}")));
    assert!(!stdout(&output).contains("session=abc"));
    assert!(server.requests()[0].starts_with("POST /cookie"));
}

#[test]
fn send_reads_raw_http_files_and_reports_failing_status() {
    let server = router();
    let raw = format!("GET /missing HTTP/1.1\r\nHost: {}\r\n\r\n", server.address);
    let path = scratch("request.http", &raw);
    let output = requestor(&["send", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).starts_with("HTTP/1.1 404 Not Found"));
}

#[test]
fn import_prints_postman_collection_json() {
    let path = scratch("import.txt", "curl -X PUT http://example.test/items -d x");
    let output = requestor(&["import", path.to_str().unwrap(), "--to", "postman"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("collection/v2.1.0"));
    assert!(text.contains("\"PUT\""));
    assert_eq!(
        requestor(&["import", path.to_str().unwrap()]).status.code(),
        Some(2)
    );
}

#[test]
fn help_and_version_succeed() {
    let help = requestor(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(stdout(&help).contains("USAGE:"));
    let version = requestor(&["--version"]);
    assert_eq!(
        stdout(&version).trim(),
        concat!("requestor ", env!("CARGO_PKG_VERSION"))
    );
}

fn secret_environment(server: &TestServer) -> PathBuf {
    scratch(
        "secret-env.json",
        &format!(
            r#"{{"name":"E","values":[
                {{"key":"base","value":"http://{}","type":"default"}},
                {{"key":"apiToken","value":"{SECRET_TOKEN}","type":"secret"}}]}}"#,
            server.address
        ),
    )
}

#[test]
fn secret_variables_are_masked_wherever_they_land_in_the_url() {
    let server = router();
    let environment = secret_environment(&server);
    let document = scratch(
        "secret-vars.json",
        r#"{"info":{"name":"s"},"item":[
            {"name":"Query","request":"{{base}}/ok?q={{apiToken}}"},
            {"name":"Path","request":"{{base}}/ok/{{apiToken}}/x"}]}"#,
    );
    for json in [false, true] {
        let mut arguments = vec!["run", document.to_str().unwrap()];
        arguments.extend(["--env", environment.to_str().unwrap()]);
        if json {
            arguments.push("--json");
        }
        let output = requestor(&arguments);
        let text = stdout(&output);
        assert!(!text.contains(SECRET_TOKEN), "{text}");
        assert!(!stderr(&output).contains(SECRET_TOKEN));
        assert!(text.contains(&format!("q={REDACTED}")), "{text}");
        assert!(text.contains(&format!("/ok/{REDACTED}/x")), "{text}");
    }
}

#[test]
fn folder_variables_and_inherited_noauth_apply_to_requests() {
    let server = router();
    let document = scratch(
        "folder-vars.json",
        &format!(
            r#"{{"info":{{"name":"f"}},
                "auth":{{"type":"bearer","bearer":[{{"key":"token","value":"{SECRET_TOKEN}"}}]}},
                "item":[{{"name":"G","variable":[{{"key":"path","value":"/ok"}}],
                    "item":[{{"name":"One","request":"http://{}{{{{path}}}}"}}]}}]}}"#,
            server.address
        ),
    );
    let output = requestor(&["run", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(server.requests()[0].contains(&format!("Bearer {SECRET_TOKEN}")));
    assert!(!stdout(&output).contains(SECRET_TOKEN));
}

#[test]
fn urls_with_credentials_are_masked_in_output() {
    let server = router();
    let document = scratch(
        "userinfo.json",
        &format!(
            r#"{{"info":{{"name":"u"}},"item":[{{"name":"One","request":"http://user:pw@{}/ok"}}]}}"#,
            server.address
        ),
    );
    let output = requestor(&["run", document.to_str().unwrap()]);
    let text = stdout(&output);
    assert!(text.contains(&format!("http://{REDACTED}@")), "{text}");
    assert!(!text.contains("user:pw"));
}

#[test]
fn a_request_that_cannot_be_prepared_is_a_failed_request() {
    let document = scratch(
        "no-url.json",
        r#"{"info":{"name":"n"},"item":[{"name":"Blank","request":{"method":"GET"}}]}"#,
    );
    let output = requestor(&["run", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("FAILED"));
}

#[cfg(unix)]
#[test]
fn a_closed_stdout_exits_quietly_with_success() {
    use std::process::Stdio;

    let server = router();
    let path = collection(&server);
    let mut child = Command::new(env!("CARGO_BIN_EXE_requestor"))
        .args(["run", path.to_str().unwrap(), "--request", "Good"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(stderr(&output).is_empty(), "{}", stderr(&output));
}

#[test]
fn send_accepts_options_after_the_target() {
    let server = router();
    let command = format!("curl {}", server.url("/ok"));
    let output = requestor(&["send", &command, "--timeout-ms", "5000"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

#[test]
fn help_flag_works_after_subcommands() {
    for arguments in [["run", "-h"], ["send", "--help"], ["import", "--help"]] {
        let output = requestor(&arguments);
        assert_eq!(output.status.code(), Some(0));
        assert!(stdout(&output).contains("USAGE:"));
    }
}

#[test]
fn import_converts_openapi_documents_and_forwards_warnings() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../core/tests/fixtures/petstore_openapi.json"
    );
    let output = requestor(&["import", fixture, "--to", "postman"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("collection/v2.1.0"));
    assert!(text.contains("pets"));
}
