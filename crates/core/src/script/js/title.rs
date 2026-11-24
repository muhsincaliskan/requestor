use serde_json::Value;

use super::super::literal::quote;
use crate::script::{Comparison, Condition, Operand, Subject};

impl Condition {
    pub fn title(&self) -> String {
        match self {
            Condition::Compare {
                subject,
                comparison,
                operand: Some(Operand::Literal(value)),
            } => classic_title(subject, *comparison, value).unwrap_or_else(|| self.text()),
            _ => self.text(),
        }
    }
}

fn classic_title(subject: &Subject, comparison: Comparison, value: &Value) -> Option<String> {
    match (subject, comparison, value) {
        (Subject::Status, Comparison::Is, Value::Number(code)) => Some(format!("Status is {code}")),
        (Subject::Time, Comparison::Below, Value::Number(limit)) => {
            Some(format!("Response time is below {limit} ms"))
        }
        (Subject::Body, Comparison::Contains, Value::String(text)) => {
            Some(format!("Body contains {}", quote(text)))
        }
        (Subject::Header(name), Comparison::Contains, Value::String(text)) => {
            Some(format!("Header {name} contains {}", quote(text)))
        }
        (Subject::Json(path), Comparison::Is, expected) => {
            Some(format!("JSON {} equals {expected}", path.text()))
        }
        _ => None,
    }
}
