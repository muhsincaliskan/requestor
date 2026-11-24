use requestor_core::prepared::PreparedRequest;
use requestor_core::response::HttpResponse;
use requestor_core::script::CheckOutcome;
use requestor_core::secret::Redactor;

use crate::json::JsonLine;

const UNKNOWN: &str = "-";
const PASSED_MARK: &str = "  ✓ ";
const FAILED_MARK: &str = "  ✗ ";

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Responded {
        status: u16,
        is_error: bool,
        elapsed_ms: u128,
        bytes: usize,
    },
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestLine {
    method: String,
    url: String,
}

impl RequestLine {
    pub fn from(request: &PreparedRequest, redactor: &Redactor) -> Self {
        RequestLine {
            method: request.method.as_str().to_string(),
            url: redactor.redact_url(&request.url),
        }
    }

    fn unknown() -> Self {
        RequestLine {
            method: UNKNOWN.to_string(),
            url: UNKNOWN.to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RequestReport {
    name: String,
    line: RequestLine,
    outcome: Outcome,
    checks: Vec<CheckOutcome>,
}

impl RequestReport {
    pub fn from_response(name: &str, line: &RequestLine, response: &HttpResponse) -> Self {
        RequestReport {
            name: name.to_string(),
            line: line.clone(),
            outcome: Outcome::Responded {
                status: response.status,
                is_error: response.is_client_or_server_error(),
                elapsed_ms: response.elapsed.as_millis(),
                bytes: response.body.len(),
            },
            checks: Vec::new(),
        }
    }

    pub fn with_checks(mut self, checks: Vec<CheckOutcome>) -> Self {
        self.checks = checks;
        self
    }

    pub fn failed(name: &str, line: Option<&RequestLine>, reason: impl ToString) -> Self {
        RequestReport {
            name: name.to_string(),
            line: line.cloned().unwrap_or_else(RequestLine::unknown),
            outcome: Outcome::Failed(reason.to_string()),
            checks: Vec::new(),
        }
    }

    pub fn is_success(&self) -> bool {
        let responded = matches!(
            self.outcome,
            Outcome::Responded {
                is_error: false,
                ..
            }
        );
        responded && self.checks.iter().all(CheckOutcome::passed)
    }

    fn check_lines(&self) -> String {
        self.checks
            .iter()
            .map(|check| match &check.failure {
                None => format!("\n{PASSED_MARK}{}", check.title),
                Some(failure) => format!("\n{FAILED_MARK}{}: {failure}", check.title),
            })
            .collect()
    }

    fn failed_checks(&self) -> Vec<String> {
        self.checks
            .iter()
            .filter_map(|check| {
                let failure = check.failure.as_ref()?;
                Some(format!("{}: {failure}", check.title))
            })
            .collect()
    }

    pub fn to_text(&self) -> String {
        let head = format!("{}: {} {}", self.name, self.line.method, self.line.url);
        match &self.outcome {
            Outcome::Responded {
                status,
                elapsed_ms,
                bytes,
                ..
            } => format!(
                "{head} -> {status} {elapsed_ms} ms {bytes} B{}",
                self.check_lines()
            ),
            Outcome::Failed(reason) => format!("{head} -> FAILED {reason}"),
        }
    }

    pub fn to_json(&self) -> String {
        let line = JsonLine::new()
            .text("name", &self.name)
            .text("method", &self.line.method)
            .text("url", &self.line.url)
            .raw("ok", self.is_success());
        let line = match &self.outcome {
            Outcome::Responded {
                status,
                elapsed_ms,
                bytes,
                ..
            } => line
                .raw("status", status)
                .raw("elapsed_ms", elapsed_ms)
                .raw("bytes", bytes),
            Outcome::Failed(reason) => line.text("error", reason),
        };
        if self.checks.is_empty() {
            return line.render();
        }
        let failed = self.failed_checks();
        line.raw("checks_passed", self.checks.len() - failed.len())
            .texts("checks_failed", &failed)
            .render()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use requestor_core::collection::{Request, Url};
    use requestor_core::secret::REDACTED;
    use requestor_core::settings::RequestSettings;
    use requestor_core::variables::VariableScope;

    use super::*;

    fn prepared() -> PreparedRequest {
        let request = Request::new("GET", Url::Raw("http://h/p?token=s3cret".to_string()));
        PreparedRequest::from_request(&request, &VariableScope::new(), &RequestSettings::default())
            .unwrap()
    }

    fn line() -> RequestLine {
        RequestLine::from(&prepared(), &Redactor::default())
    }

    fn response(status: u16) -> HttpResponse {
        HttpResponse {
            status,
            reason: String::new(),
            version: String::new(),
            headers: Vec::new(),
            body: vec![0; 5],
            truncated: false,
            elapsed: Duration::from_millis(12),
            timing: None,
        }
    }

    #[test]
    fn success_covers_2xx_and_3xx() {
        let statuses = [
            (200, true),
            (302, true),
            (399, true),
            (400, false),
            (500, false),
        ];
        for (status, expected) in statuses {
            let report = RequestReport::from_response("n", &line(), &response(status));
            assert_eq!(report.is_success(), expected, "status {status}");
        }
    }

    #[test]
    fn text_line_shows_method_url_status_time_and_size_without_secrets() {
        let report = RequestReport::from_response("List", &line(), &response(200));
        assert_eq!(
            report.to_text(),
            format!("List: GET http://h/p?token={REDACTED} -> 200 12 ms 5 B")
        );
    }

    #[test]
    fn json_line_describes_responses_and_failures() {
        let report = RequestReport::from_response("List", &line(), &response(404));
        assert_eq!(
            report.to_json(),
            format!(
                r#"{{"name":"List","method":"GET","url":"http://h/p?token={REDACTED}","ok":false,"status":404,"elapsed_ms":12,"bytes":5}}"#
            )
        );
        let failure = RequestReport::failed("Boom", None, "the request timed out");
        assert!(!failure.is_success());
        assert_eq!(
            failure.to_json(),
            r#"{"name":"Boom","method":"-","url":"-","ok":false,"error":"the request timed out"}"#
        );
    }
}
