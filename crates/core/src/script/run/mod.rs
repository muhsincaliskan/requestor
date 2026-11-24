mod compare;
mod observe;

use std::collections::HashMap;
use std::ops::ControlFlow;

use serde_json::Value;

pub(crate) use compare::text_of;

use super::report::{CheckOutcome, ScriptReport, VariableWrite, Written};
use super::{
    Condition, Conditional, Operand, Script, SetValue, Statement, Subject, VariableTarget,
};
use crate::response::HttpResponse;
use crate::variables::VariableScope;

const CONDITION_FALSE: &str = "condition was false";
const SCRIPT_STOPPED: &str = "script stopped here";

pub struct ScriptContext<'a> {
    pub response: Option<&'a HttpResponse>,
    pub variables: &'a VariableScope,
}

impl Script {
    pub fn run(&self, context: &ScriptContext) -> ScriptReport {
        let mut runner = Runner {
            context,
            json: None,
            written: HashMap::new(),
            report: ScriptReport::default(),
        };
        let _ = runner.block(&self.statements);
        runner.report
    }
}

struct Runner<'a, 'b> {
    context: &'a ScriptContext<'b>,
    json: Option<Result<Value, String>>,
    written: HashMap<String, Option<String>>,
    report: ScriptReport,
}

impl Runner<'_, '_> {
    fn block(&mut self, statements: &[Statement]) -> ControlFlow<()> {
        statements
            .iter()
            .try_for_each(|statement| self.statement(statement))
    }

    fn statement(&mut self, statement: &Statement) -> ControlFlow<()> {
        match statement {
            Statement::Blank | Statement::Comment(_) => {}
            Statement::Set {
                variable,
                value,
                target,
            } => {
                let text = value.produce();
                self.write(variable, *target, Ok(Written::Value(text)));
            }
            Statement::Clear { variable, target } => {
                self.write(variable, *target, Ok(Written::Removed))
            }
            Statement::Expect { condition, name } => self.expect(condition, name.as_deref()),
            Statement::Save {
                subject,
                variable,
                target,
            } => {
                let value = self.save_value(subject);
                self.write(variable, *target, value);
            }
            Statement::Fail(message) => return self.fail(message),
            Statement::If(conditional) => return self.conditional(conditional),
        }
        ControlFlow::Continue(())
    }

    fn conditional(&mut self, conditional: &Conditional) -> ControlFlow<()> {
        let chosen = conditional
            .branches
            .iter()
            .find(|branch| self.evaluate(&branch.condition).unwrap_or(false))
            .map(|branch| branch.body.as_slice())
            .or(conditional.otherwise.as_deref());
        chosen.map_or(ControlFlow::Continue(()), |body| self.block(body))
    }

    fn expect(&mut self, condition: &Condition, name: Option<&str>) {
        let failure = match self.evaluate(condition) {
            Ok(true) => None,
            Ok(false) => Some(self.explain(condition)),
            Err(reason) => Some(reason),
        };
        self.report.checks.push(CheckOutcome {
            title: name.map_or_else(|| condition.title(), str::to_string),
            failure,
        });
    }

    fn fail(&mut self, message: &str) -> ControlFlow<()> {
        self.report.checks.push(CheckOutcome {
            title: message.to_string(),
            failure: Some(SCRIPT_STOPPED.to_string()),
        });
        self.report.failure = Some(message.to_string());
        ControlFlow::Break(())
    }

    fn write(&mut self, variable: &str, target: VariableTarget, value: Result<Written, String>) {
        if let Ok(written) = &value {
            let text = match written {
                Written::Value(text) => Some(text.clone()),
                Written::Removed => None,
            };
            self.written.insert(variable.to_string(), text);
        }
        self.report.writes.push(VariableWrite {
            variable: variable.to_string(),
            target,
            value,
        });
    }

    fn save_value(&mut self, subject: &Subject) -> Result<Written, String> {
        match self.observe(subject)? {
            Some(value) => Ok(Written::Value(text_of(&value))),
            None => Err(subject.missing_reason().to_string()),
        }
    }

    fn evaluate(&mut self, condition: &Condition) -> Result<bool, String> {
        match condition {
            Condition::Compare {
                subject,
                comparison,
                operand,
            } => {
                let observed = self.observe(subject)?;
                let operand = operand
                    .as_ref()
                    .map(|operand| self.operand(operand))
                    .transpose()?;
                Ok(compare::holds(
                    *comparison,
                    observed.as_ref(),
                    operand.as_ref(),
                ))
            }
            Condition::Not(inner) => self.evaluate(inner).map(|holds| !holds),
            Condition::All(parts) => self.every(parts, true),
            Condition::Any(parts) => self.every(parts, false).map(|none| !none),
        }
    }

    fn every(&mut self, parts: &[Condition], expected: bool) -> Result<bool, String> {
        for part in parts {
            if self.evaluate(part)? != expected {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn explain(&mut self, condition: &Condition) -> String {
        let Condition::Compare { subject, .. } = condition else {
            return CONDITION_FALSE.to_string();
        };
        match self.observe(subject) {
            Ok(Some(value)) => format!("got {}", subject.shown(&value)),
            Ok(None) => subject.missing_reason().to_string(),
            Err(reason) => reason,
        }
    }

    fn operand(&self, operand: &Operand) -> Result<Value, String> {
        match operand {
            Operand::Literal(value) => Ok(value.clone()),
            Operand::Variable(name) => self
                .variable(name)
                .map(Value::String)
                .ok_or_else(|| format!("{{{{{name}}}}} is not defined")),
        }
    }
}

impl SetValue {
    fn produce(&self) -> String {
        match self {
            SetValue::Literal(value) => text_of(value),
            SetValue::Dynamic(dynamic) => dynamic.generate(),
        }
    }
}

#[cfg(test)]
mod tests;
