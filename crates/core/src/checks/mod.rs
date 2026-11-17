mod apply;
mod evaluate;
mod events;
mod literal;
mod path;
mod script;

use serde_json::Value;

pub use evaluate::{CheckOutcome, Extracted, RulesReport};
pub use path::JsonPath;

#[derive(Debug, Clone, PartialEq)]
pub enum Check {
    Status(u16),
    ResponseTimeBelow(u64),
    BodyContains(String),
    HeaderContains { name: String, text: String },
    JsonEquals { path: JsonPath, expected: Value },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractSource {
    Json(JsonPath),
    Header(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VariableTarget {
    #[default]
    Environment,
    Collection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extraction {
    pub variable: String,
    pub source: ExtractSource,
    pub target: VariableTarget,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResponseRules {
    pub checks: Vec<Check>,
    pub extractions: Vec<Extraction>,
}

impl ResponseRules {
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty() && self.extractions.is_empty()
    }
}
