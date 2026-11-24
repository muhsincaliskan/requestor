use serde_json::{Number, Value};

use super::super::syntax::Expr;
use super::Recognizer;
use crate::script::dynamic::MILLIS_PER_SECOND;
use crate::script::{Dynamic, JsonPath, Operand, Subject};

const STATUS_PATHS: [&str; 2] = ["pm.response.code", "responseCode.code"];
const TIME_PATHS: [&str; 2] = ["pm.response.responseTime", "responseTime"];
const LEGACY_BODY: &str = "responseBody";
const HEADER_CALLS: [&str; 2] = ["pm.response.headers.get", "postman.getResponseHeader"];
const VARIABLE_CALLS: [&str; 4] = [
    "pm.variables.get",
    "pm.environment.get",
    "pm.collectionVariables.get",
    "postman.getEnvironmentVariable",
];
const LENGTH: &str = "length";

impl Recognizer {
    pub(super) fn subject(&self, expression: &Expr) -> Option<Subject> {
        let expression = expression.ungrouped();
        let path = expression.path();
        if STATUS_PATHS
            .iter()
            .any(|known| path.as_deref() == Some(known))
        {
            return Some(Subject::Status);
        }
        if TIME_PATHS
            .iter()
            .any(|known| path.as_deref() == Some(known))
        {
            return Some(Subject::Time);
        }
        if path.as_deref() == Some(LEGACY_BODY)
            || expression.call_of("pm.response.text") == Some(&[])
        {
            return Some(Subject::Body);
        }
        if let Some(name) = single_text(expression, &HEADER_CALLS) {
            return Some(Subject::Header(name));
        }
        if let Some(name) = single_text(expression, &VARIABLE_CALLS) {
            return Some(Subject::Variable(name));
        }
        self.json_path(expression).map(Subject::Json)
    }

    pub(super) fn json_path(&self, expression: &Expr) -> Option<JsonPath> {
        match expression.ungrouped() {
            Expr::Name(name) => self.aliases.get(name).cloned(),
            Expr::Member(_, field) if field == LENGTH => None,
            Expr::Member(object, field) => Some(self.json_path(object)?.with_key(field)),
            Expr::Index(object, index) => {
                let base = self.json_path(object)?;
                match index.as_ref() {
                    Expr::Text(key) => Some(base.with_key(key)),
                    Expr::Number(number) => {
                        Some(base.with_index(usize::try_from(number.as_u64()?).ok()?))
                    }
                    _ => None,
                }
            }
            call => json_root(call).then(JsonPath::default),
        }
    }

    pub(super) fn operand(&self, expression: &Expr) -> Option<Operand> {
        if let Some(name) = single_text(expression, &VARIABLE_CALLS) {
            return Some(Operand::Variable(name));
        }
        literal(expression).map(Operand::Literal)
    }

    pub(super) fn operand_for(&self, subject: &Subject, expression: &Expr) -> Option<Operand> {
        let operand = self.operand(expression)?;
        let numeric = matches!(subject, Subject::Status | Subject::Time);
        match &operand {
            Operand::Literal(value) if numeric && !value.is_number() => None,
            _ => Some(operand),
        }
    }
}

fn json_root(expression: &Expr) -> bool {
    if expression.call_of("pm.response.json") == Some(&[]) {
        return true;
    }
    match expression.call_of("JSON.parse") {
        Some([argument]) => {
            argument.path().as_deref() == Some(LEGACY_BODY)
                || argument.call_of("pm.response.text") == Some(&[])
        }
        _ => false,
    }
}

fn single_text(expression: &Expr, calls: &[&str]) -> Option<String> {
    calls
        .iter()
        .find_map(|call| match expression.call_of(call)? {
            [Expr::Text(name)] => Some(name.clone()),
            _ => None,
        })
}

pub(super) fn literal(expression: &Expr) -> Option<Value> {
    match expression.ungrouped() {
        Expr::Text(text) => Some(Value::String(text.clone())),
        Expr::Number(number) => Some(Value::Number(number.clone())),
        Expr::Negate(inner) => match inner.as_ref() {
            Expr::Number(number) => negated(number).map(Value::Number),
            _ => None,
        },
        Expr::Name(name) => match name.as_str() {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            "null" => Some(Value::Null),
            _ => None,
        },
        _ => None,
    }
}

fn negated(number: &Number) -> Option<Number> {
    serde_json::from_str(&format!("-{number}")).ok()
}

pub(super) fn dynamic(expression: &Expr) -> Option<Dynamic> {
    if let Some([Expr::Text(text)]) = expression.call_of("pm.variables.replaceIn") {
        let name = text.strip_prefix("{{")?.strip_suffix("}}")?;
        return Dynamic::from_placeholder(name);
    }
    if expression.call_of("Date.now") == Some(&[]) {
        return Some(Dynamic::TimestampMillis);
    }
    if let Some([min, max]) = expression.call_of("_.random") {
        return Some(Dynamic::RandomNumber {
            min: whole(min)?,
            max: whole(max)?,
        });
    }
    if let Some((object, [])) = expression.method_call("toISOString") {
        return matches!(object, Expr::New(call) if call.call_of("Date") == Some(&[]))
            .then_some(Dynamic::NowIso);
    }
    seconds_timestamp(expression).then_some(Dynamic::Timestamp)
}

fn seconds_timestamp(expression: &Expr) -> bool {
    let Some([argument]) = expression.call_of("Math.floor") else {
        return false;
    };
    let Expr::Binary("/", left, right) = argument.ungrouped() else {
        return false;
    };
    left.call_of("Date.now") == Some(&[])
        && matches!(right.as_ref(), Expr::Number(number) if number.as_u64() == Some(MILLIS_PER_SECOND))
}

fn whole(expression: &Expr) -> Option<i64> {
    literal(expression)?.as_i64()
}
