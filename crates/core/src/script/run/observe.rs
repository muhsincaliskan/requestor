use serde_json::Value;

use super::compare::text_of;
use super::Runner;
use crate::response::HttpResponse;
use crate::script::Subject;

const NO_RESPONSE: &str = "there is no response yet";
const NOT_JSON: &str = "body is not JSON";
const MISSING_HEADER: &str = "header is missing";
const MISSING_VALUE: &str = "no value at this path";
const MISSING_VARIABLE: &str = "variable is not defined";

impl Runner<'_, '_> {
    pub(super) fn observe(&mut self, subject: &Subject) -> Result<Option<Value>, String> {
        if let Subject::Variable(name) = subject {
            return Ok(self.variable(name).map(Value::String));
        }
        let response = self.context.response.ok_or(NO_RESPONSE)?;
        Ok(match subject {
            Subject::Status => Some(Value::from(response.status)),
            Subject::Time => Some(Value::from(elapsed_millis(response))),
            Subject::Body => Some(Value::String(response.body_text().into_owned())),
            Subject::Header(name) => response
                .header(name)
                .map(|value| Value::String(value.to_string())),
            Subject::Json(path) => path.find(self.json(response)?).cloned(),
            Subject::Variable(_) => None,
        })
    }

    pub(super) fn variable(&self, name: &str) -> Option<String> {
        let scope = self.context.variables;
        match self.written.get(name) {
            Some(written) => written.clone(),
            None => scope.lookup(name).map(|raw| scope.resolve(raw).text),
        }
    }

    fn json(&mut self, response: &HttpResponse) -> Result<&Value, String> {
        let parsed = self.json.get_or_insert_with(|| {
            serde_json::from_slice(&response.body).map_err(|_| NOT_JSON.to_string())
        });
        parsed.as_ref().map_err(Clone::clone)
    }
}

fn elapsed_millis(response: &HttpResponse) -> u64 {
    u64::try_from(response.elapsed.as_millis()).unwrap_or(u64::MAX)
}

impl Subject {
    pub(super) fn missing_reason(&self) -> &'static str {
        match self {
            Subject::Header(_) => MISSING_HEADER,
            Subject::Variable(_) => MISSING_VARIABLE,
            _ => MISSING_VALUE,
        }
    }

    pub(super) fn shown(&self, value: &Value) -> String {
        match self {
            Subject::Json(_) => value.to_string(),
            _ => text_of(value),
        }
    }
}
