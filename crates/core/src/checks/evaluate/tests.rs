use std::time::Duration;

use serde_json::json;

use crate::checks::{Check, ExtractSource, Extraction, JsonPath, ResponseRules, VariableTarget};
use crate::header::Header;
use crate::response::HttpResponse;

fn response(body: &str) -> HttpResponse {
    HttpResponse {
        status: 201,
        reason: "Created".to_string(),
        version: "HTTP/1.1".to_string(),
        headers: vec![Header::new("Content-Type", "application/json")],
        body: body.as_bytes().to_vec(),
        truncated: false,
        elapsed: Duration::from_millis(120),
    }
}

fn path(text: &str) -> JsonPath {
    JsonPath::parse(text).unwrap()
}

#[test]
fn checks_report_what_they_saw_when_they_fail() {
    let rules = ResponseRules {
        checks: vec![
            Check::Status(201),
            Check::Status(200),
            Check::ResponseTimeBelow(100),
            Check::BodyContains("ada".to_string()),
            Check::HeaderContains {
                name: "content-type".to_string(),
                text: "xml".to_string(),
            },
            Check::JsonEquals {
                path: path("user.id"),
                expected: json!(7),
            },
            Check::JsonEquals {
                path: path("user.age"),
                expected: json!(1),
            },
        ],
        extractions: Vec::new(),
    };
    let report = rules.evaluate(&response(r#"{"user":{"id":7,"name":"ada"}}"#));
    let failures: Vec<_> = report
        .checks
        .iter()
        .map(|outcome| outcome.failure.as_deref())
        .collect();
    assert_eq!(
        failures,
        [
            None,
            Some("got 201"),
            Some("took 120 ms"),
            None,
            Some("got application/json"),
            None,
            Some("no value at this path"),
        ]
    );
    assert_eq!(report.passed_count(), 3);
    assert!(!report.all_passed());
}

#[test]
fn extractions_read_strings_numbers_and_headers() {
    let extraction = |variable: &str, source| Extraction {
        variable: variable.to_string(),
        source,
        target: VariableTarget::Environment,
    };
    let rules = ResponseRules {
        checks: Vec::new(),
        extractions: vec![
            extraction("token", ExtractSource::Json(path("auth.token"))),
            extraction("id", ExtractSource::Json(path("auth.id"))),
            extraction("kind", ExtractSource::Header("Content-Type".to_string())),
            extraction("missing", ExtractSource::Json(path("auth.nope"))),
        ],
    };
    let report = rules.evaluate(&response(r#"{"auth":{"token":"t-1","id":42}}"#));
    let values: Vec<_> = report
        .extracted
        .iter()
        .map(|extracted| extracted.value.clone())
        .collect();
    assert_eq!(
        values,
        [
            Ok("t-1".to_string()),
            Ok("42".to_string()),
            Ok("application/json".to_string()),
            Err("no value at this path".to_string()),
        ]
    );
}
