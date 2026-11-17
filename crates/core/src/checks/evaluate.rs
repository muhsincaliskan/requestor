use serde_json::Value;

use super::{Check, ExtractSource, Extraction, ResponseRules};
use crate::response::HttpResponse;
use crate::secret::Redactor;

const MISSING_HEADER: &str = "header is missing";
const MISSING_VALUE: &str = "no value at this path";
const NOT_JSON: &str = "body is not JSON";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    pub title: String,
    pub failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Extracted {
    pub extraction: Extraction,
    pub value: Result<String, String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RulesReport {
    pub checks: Vec<CheckOutcome>,
    pub extracted: Vec<Extracted>,
}

struct Observed<'a> {
    response: &'a HttpResponse,
    json: Option<Value>,
}

impl ResponseRules {
    pub fn evaluate(&self, response: &HttpResponse) -> RulesReport {
        let observed = Observed {
            response,
            json: self
                .reads_json()
                .then(|| serde_json::from_slice(&response.body).ok())
                .flatten(),
        };
        RulesReport {
            checks: self
                .checks
                .iter()
                .map(|check| check.evaluate(&observed))
                .collect(),
            extracted: self
                .extractions
                .iter()
                .map(|extraction| Extracted {
                    extraction: extraction.clone(),
                    value: extraction.source.read(&observed),
                })
                .collect(),
        }
    }
}

impl ResponseRules {
    fn reads_json(&self) -> bool {
        self.checks
            .iter()
            .any(|check| matches!(check, Check::JsonEquals { .. }))
            || self
                .extractions
                .iter()
                .any(|extraction| matches!(extraction.source, ExtractSource::Json(_)))
    }
}

impl CheckOutcome {
    pub fn passed(&self) -> bool {
        self.failure.is_none()
    }

    pub fn redacted(self, redactor: &Redactor) -> Self {
        CheckOutcome {
            failure: self.failure.map(|failure| redactor.redact_text(&failure)),
            ..self
        }
    }
}

impl RulesReport {
    pub fn passed_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|outcome| outcome.passed())
            .count()
    }

    pub fn all_passed(&self) -> bool {
        self.passed_count() == self.checks.len()
    }
}

impl Check {
    fn evaluate(&self, observed: &Observed) -> CheckOutcome {
        CheckOutcome {
            title: self.title(),
            failure: self.failure(observed),
        }
    }

    fn failure(&self, observed: &Observed) -> Option<String> {
        let response = observed.response;
        match self {
            Check::Status(code) => {
                (response.status != *code).then(|| format!("got {}", response.status))
            }
            Check::ResponseTimeBelow(limit) => {
                let took = response.elapsed.as_millis();
                (took >= u128::from(*limit)).then(|| format!("took {took} ms"))
            }
            Check::BodyContains(text) => {
                (!response.body_text().contains(text.as_str())).then(|| "not found".to_string())
            }
            Check::HeaderContains { name, text } => match response.header(name) {
                None => Some(MISSING_HEADER.to_string()),
                Some(value) => (!value.contains(text.as_str())).then(|| format!("got {value}")),
            },
            Check::JsonEquals { path, expected } => {
                let Some(json) = observed.json.as_ref() else {
                    return Some(NOT_JSON.to_string());
                };
                match path.find(json) {
                    None => Some(MISSING_VALUE.to_string()),
                    Some(found) => (found != expected).then(|| format!("got {found}")),
                }
            }
        }
    }
}

impl ExtractSource {
    fn read(&self, observed: &Observed) -> Result<String, String> {
        match self {
            ExtractSource::Header(name) => observed
                .response
                .header(name)
                .map(str::to_string)
                .ok_or_else(|| MISSING_HEADER.to_string()),
            ExtractSource::Json(path) => {
                let json = observed.json.as_ref().ok_or_else(|| NOT_JSON.to_string())?;
                match path.find(json) {
                    Some(Value::String(text)) => Ok(text.clone()),
                    Some(other) => Ok(other.to_string()),
                    None => Err(MISSING_VALUE.to_string()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
