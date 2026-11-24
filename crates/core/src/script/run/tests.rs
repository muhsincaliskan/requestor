use std::time::Duration;

use crate::header::Header;
use crate::response::HttpResponse;
use crate::script::{Phase, Script, ScriptContext, ScriptReport, VariableTarget, Written};
use crate::variables::{VariableLayer, VariableScope};

fn response(status: u16, body: &str) -> HttpResponse {
    HttpResponse {
        status,
        reason: String::new(),
        version: "HTTP/1.1".to_string(),
        headers: vec![Header::new("Content-Type", "application/json")],
        body: body.as_bytes().to_vec(),
        truncated: false,
        elapsed: Duration::from_millis(120),
        timing: None,
    }
}

fn run(text: &str, response: Option<&HttpResponse>) -> ScriptReport {
    let phase = if response.is_some() {
        Phase::AfterResponse
    } else {
        Phase::BeforeRequest
    };
    let scope = VariableScope::new().with_layer(VariableLayer::default().with("env", "prod"));
    Script::parse(text, phase).unwrap().run(&ScriptContext {
        response,
        variables: &scope,
    })
}

#[test]
fn expectations_pass_and_fail_with_reasons() {
    let answer = response(404, r#"{"data": {"items": [1, 2]}}"#);
    let report = run(
        "expect status is 200\nexpect time below 500 ms\nexpect json data.items contains 2\nexpect header X-Missing exists",
        Some(&answer),
    );
    let failures: Vec<Option<&str>> = report
        .checks
        .iter()
        .map(|check| check.failure.as_deref())
        .collect();
    assert_eq!(
        failures,
        vec![Some("got 404"), None, None, Some("header is missing")]
    );
    assert_eq!(report.checks[0].title, "Status is 200");
    assert_eq!(report.passed_count(), 2);
}

#[test]
fn saved_values_and_branches_follow_the_response() {
    let answer = response(200, r#"{"data": {"token": "t-1", "count": 3}}"#);
    let report = run(
        "if status is 200\n  save json data.token as token\n  save json data.count as count in collection\nelse\n  clear token\nend\nsave json data.nope as lost",
        Some(&answer),
    );
    let values: Vec<_> = report
        .writes
        .iter()
        .map(|write| (write.variable.as_str(), write.target, write.value.clone()))
        .collect();
    assert_eq!(
        values,
        vec![
            (
                "token",
                VariableTarget::Environment,
                Ok(Written::Value("t-1".into()))
            ),
            (
                "count",
                VariableTarget::Collection,
                Ok(Written::Value("3".into()))
            ),
            (
                "lost",
                VariableTarget::Environment,
                Err("no value at this path".into())
            ),
        ]
    );
}

#[test]
fn later_lines_see_earlier_writes_and_fail_stops_the_script() {
    let report = run(
        "set a to 5 in request\nif {{a}} is 5 and {{env}} is \"prod\"\n  fail \"stop here\"\nend\nset never to 1",
        None,
    );
    assert_eq!(report.failure.as_deref(), Some("stop here"));
    assert_eq!(report.writes.len(), 1);
    assert_eq!(
        report.request_variables(),
        vec![("a".to_string(), "5".to_string())]
    );
    assert!(!report.all_passed());
}

#[test]
fn non_json_bodies_fail_json_checks_and_skip_json_branches() {
    let answer = response(200, "plain");
    let report = run(
        "expect json id is 1\nif json id is 1\n  set x to 1\nend",
        Some(&answer),
    );
    assert_eq!(
        report.checks[0].failure.as_deref(),
        Some("body is not JSON")
    );
    assert!(report.writes.is_empty());
}

#[test]
fn dynamic_values_are_generated_when_the_script_runs() {
    let report = run("set id to new uuid\nset n to random number 5 5", None);
    let Ok(Written::Value(id)) = &report.writes[0].value else {
        panic!("uuid written");
    };
    assert_eq!(id.len(), 36);
    assert_eq!(report.writes[1].value, Ok(Written::Value("5".into())));
}
