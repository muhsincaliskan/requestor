use serde_json::Value;

use crate::checks::{Check, ExtractSource, Extraction, JsonPath, VariableTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckKind {
    #[default]
    Status,
    ResponseTime,
    BodyContains,
    HeaderContains,
    JsonEquals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtractKind {
    #[default]
    Json,
    Header,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckRow {
    pub kind: CheckKind,
    pub subject: String,
    pub expected: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractRow {
    pub variable: String,
    pub kind: ExtractKind,
    pub source: String,
    pub target: VariableTarget,
}

impl CheckKind {
    pub const ALL: [CheckKind; 5] = [
        CheckKind::Status,
        CheckKind::ResponseTime,
        CheckKind::BodyContains,
        CheckKind::HeaderContains,
        CheckKind::JsonEquals,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CheckKind::Status => "Status is",
            CheckKind::ResponseTime => "Response time below (ms)",
            CheckKind::BodyContains => "Body contains",
            CheckKind::HeaderContains => "Header contains",
            CheckKind::JsonEquals => "JSON value equals",
        }
    }

    pub fn expected_hint(self) -> &'static str {
        match self {
            CheckKind::Status => "200",
            CheckKind::ResponseTime => "500",
            CheckKind::BodyContains | CheckKind::HeaderContains => "Text",
            CheckKind::JsonEquals => "Value, for example 42 or ada",
        }
    }

    pub fn subject_hint(self) -> Option<&'static str> {
        match self {
            CheckKind::HeaderContains => Some("Header name"),
            CheckKind::JsonEquals => Some("Path, for example data.items[0].id"),
            _ => None,
        }
    }
}

impl ExtractKind {
    pub const ALL: [ExtractKind; 2] = [ExtractKind::Json, ExtractKind::Header];

    pub fn label(self) -> &'static str {
        match self {
            ExtractKind::Json => "JSON path",
            ExtractKind::Header => "Header",
        }
    }

    pub fn source_hint(self) -> &'static str {
        match self {
            ExtractKind::Json => "data.token",
            ExtractKind::Header => "Header name",
        }
    }
}

impl CheckRow {
    pub fn from_check(check: &Check) -> Self {
        let (kind, subject, expected) = match check {
            Check::Status(code) => (CheckKind::Status, String::new(), code.to_string()),
            Check::ResponseTimeBelow(limit) => {
                (CheckKind::ResponseTime, String::new(), limit.to_string())
            }
            Check::BodyContains(text) => (CheckKind::BodyContains, String::new(), text.clone()),
            Check::HeaderContains { name, text } => {
                (CheckKind::HeaderContains, name.clone(), text.clone())
            }
            Check::JsonEquals { path, expected } => {
                (CheckKind::JsonEquals, path.text(), expected_text(expected))
            }
        };
        CheckRow {
            kind,
            subject,
            expected,
        }
    }

    pub fn to_check(&self) -> Option<Check> {
        let expected = self.expected.trim();
        let subject = self.subject.trim();
        match self.kind {
            CheckKind::Status => expected.parse().ok().map(Check::Status),
            CheckKind::ResponseTime => expected.parse().ok().map(Check::ResponseTimeBelow),
            CheckKind::BodyContains => {
                (!self.expected.is_empty()).then(|| Check::BodyContains(self.expected.clone()))
            }
            CheckKind::HeaderContains => (!subject.is_empty()).then(|| Check::HeaderContains {
                name: subject.to_string(),
                text: self.expected.clone(),
            }),
            CheckKind::JsonEquals => Some(Check::JsonEquals {
                path: JsonPath::parse(subject)?,
                expected: expected_value(expected),
            }),
        }
    }

    pub fn is_blank(&self) -> bool {
        self.subject.is_empty() && self.expected.is_empty()
    }

    pub fn is_incomplete(&self) -> bool {
        !self.is_blank() && self.to_check().is_none()
    }
}

impl ExtractRow {
    pub fn from_extraction(extraction: &Extraction) -> Self {
        let (kind, source) = match &extraction.source {
            ExtractSource::Json(path) => (ExtractKind::Json, path.text()),
            ExtractSource::Header(name) => (ExtractKind::Header, name.clone()),
        };
        ExtractRow {
            variable: extraction.variable.clone(),
            kind,
            source,
            target: extraction.target,
        }
    }

    pub fn to_extraction(&self) -> Option<Extraction> {
        let variable = self.variable.trim();
        let source = self.source.trim();
        if variable.is_empty() {
            return None;
        }
        let source = match self.kind {
            ExtractKind::Json => ExtractSource::Json(JsonPath::parse(source)?),
            ExtractKind::Header if !source.is_empty() => ExtractSource::Header(source.to_string()),
            ExtractKind::Header => return None,
        };
        Some(Extraction {
            variable: variable.to_string(),
            source,
            target: self.target,
        })
    }

    pub fn is_blank(&self) -> bool {
        self.variable.is_empty() && self.source.is_empty()
    }

    pub fn is_incomplete(&self) -> bool {
        !self.is_blank() && self.to_extraction().is_none()
    }
}

fn expected_text(expected: &Value) -> String {
    match expected {
        Value::String(text) if expected_value(text) == *expected => text.clone(),
        other => other.to_string(),
    }
}

fn expected_value(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string()))
}
