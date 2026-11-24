mod condition;
mod normalize;
mod value;

use std::collections::HashMap;

use super::syntax::{Body, Expr, JsStatement, Located};
use crate::script::{Branch, Conditional, JsonPath, SetValue, Statement, VariableTarget};

const SETTERS: [(&str, VariableTarget); 4] = [
    ("pm.environment.set", VariableTarget::Environment),
    (
        "postman.setEnvironmentVariable",
        VariableTarget::Environment,
    ),
    ("pm.collectionVariables.set", VariableTarget::Collection),
    ("pm.variables.set", VariableTarget::Request),
];
const UNSETTERS: [(&str, VariableTarget); 4] = [
    ("pm.environment.unset", VariableTarget::Environment),
    (
        "postman.clearEnvironmentVariable",
        VariableTarget::Environment,
    ),
    ("pm.collectionVariables.unset", VariableTarget::Collection),
    ("pm.variables.unset", VariableTarget::Request),
];
const TEST_CALL: &str = "pm.test";
const LEGACY_TESTS: &str = "tests";
const ASSIGN: &str = "=";
const UNKNOWN: &str = "not a pattern Requestor can convert";

pub(super) type Failures = Vec<(usize, String)>;

#[derive(Default)]
pub(super) struct Recognizer {
    aliases: HashMap<String, JsonPath>,
}

impl Recognizer {
    pub(super) fn statements(&mut self, program: &[Located]) -> Result<Vec<Statement>, Failures> {
        let mut statements = Vec::new();
        let mut failures = Vec::new();
        for located in program {
            match self.statement(&located.statement) {
                Ok(Some(statement)) => statements.push(statement),
                Ok(None) => {}
                Err(mut inner) if !inner.is_empty() => failures.append(&mut inner),
                Err(_) => failures.push((located.first_line, UNKNOWN.to_string())),
            }
        }
        if failures.is_empty() {
            Ok(statements)
        } else {
            Err(failures)
        }
    }

    fn statement(&mut self, statement: &JsStatement) -> Result<Option<Statement>, Failures> {
        match statement {
            JsStatement::Blank => Ok(Some(Statement::Blank)),
            JsStatement::Comment(text) => Ok(Some(Statement::Comment(text.clone()))),
            JsStatement::Declare { name, value } => {
                let path = self.json_path(value).ok_or_else(Vec::new)?;
                self.aliases.insert(name.clone(), path);
                Ok(None)
            }
            JsStatement::Throw(error) => fail_message(error).map(Some).ok_or_else(Vec::new),
            JsStatement::If {
                condition,
                then,
                otherwise,
            } => self
                .conditional(condition, then, otherwise.as_deref())
                .map(Some),
            JsStatement::Expression(expression) => {
                self.expression(expression).map(Some).ok_or_else(Vec::new)
            }
        }
    }

    fn conditional(
        &mut self,
        condition: &Expr,
        then: &[Located],
        otherwise: Option<&[Located]>,
    ) -> Result<Statement, Failures> {
        let mut branches = Vec::new();
        let mut current = (condition, then);
        let mut rest = otherwise;
        loop {
            let condition = self.condition(current.0).ok_or_else(Vec::new)?;
            branches.push(Branch {
                condition,
                body: self.statements(current.1)?,
            });
            match rest {
                Some(
                    [Located {
                        statement:
                            JsStatement::If {
                                condition,
                                then,
                                otherwise,
                            },
                        ..
                    }],
                ) => {
                    current = (condition, then);
                    rest = otherwise.as_deref();
                }
                Some(block) => {
                    let otherwise = Some(self.statements(block)?);
                    return Ok(Statement::If(Conditional {
                        branches,
                        otherwise,
                    }));
                }
                None => {
                    return Ok(Statement::If(Conditional {
                        branches,
                        otherwise: None,
                    }))
                }
            }
        }
    }

    fn expression(&self, expression: &Expr) -> Option<Statement> {
        if let Some(statement) = self.variable_change(expression) {
            return Some(statement);
        }
        if let Some([title, Expr::Function(body)]) = expression.call_of(TEST_CALL) {
            return self.test(title.text()?, body);
        }
        if let Some(message) = fail_message(expression) {
            return Some(message);
        }
        self.legacy_test(expression)
    }

    fn variable_change(&self, expression: &Expr) -> Option<Statement> {
        if let Some((target, [name, value])) = find_call(expression, &SETTERS) {
            return self.assignment(name.text()?, value, target);
        }
        let (target, [name]) = find_call(expression, &UNSETTERS)? else {
            return None;
        };
        Some(Statement::Clear {
            variable: name.text()?.to_string(),
            target,
        })
    }

    fn assignment(
        &self,
        variable: &str,
        value: &Expr,
        target: VariableTarget,
    ) -> Option<Statement> {
        let variable = variable.to_string();
        if let Some(subject) = self.subject(value) {
            return Some(Statement::Save {
                subject,
                variable,
                target,
            });
        }
        let value = match value::literal(value) {
            Some(literal) => SetValue::Literal(literal),
            None => SetValue::Dynamic(value::dynamic(value)?),
        };
        Some(Statement::Set {
            variable,
            value,
            target,
        })
    }

    fn test(&self, title: &str, body: &Body) -> Option<Statement> {
        if let Some(message) = test_failure(body) {
            return Some(message);
        }
        let condition = self.test_condition(body)?;
        let name = (condition.title() != title).then(|| title.to_string());
        Some(Statement::Expect { condition, name })
    }

    fn legacy_test(&self, expression: &Expr) -> Option<Statement> {
        let Expr::Binary(ASSIGN, target, value) = expression else {
            return None;
        };
        let Expr::Index(object, name) = target.as_ref() else {
            return None;
        };
        if object.path().as_deref() != Some(LEGACY_TESTS) {
            return None;
        }
        let condition = self.condition(value)?;
        Some(Statement::Expect {
            condition,
            name: Some(name.text()?.to_string()),
        })
    }
}

fn find_call<'a>(
    expression: &'a Expr,
    calls: &[(&str, VariableTarget)],
) -> Option<(VariableTarget, &'a [Expr])> {
    calls.iter().find_map(|(path, target)| {
        expression
            .call_of(path)
            .map(|arguments| (*target, arguments))
    })
}

fn fail_message(expression: &Expr) -> Option<Statement> {
    let message = match expression {
        Expr::New(call) => match call.call_of("Error")? {
            [message] => message.text()?,
            _ => return None,
        },
        other => match other.call_of("pm.expect.fail")? {
            [message] => message.text()?,
            _ => return None,
        },
    };
    Some(Statement::Fail(message.to_string()))
}

fn test_failure(body: &Body) -> Option<Statement> {
    match body {
        Body::Expression(expression) => fail_message(expression),
        Body::Block(statements) => match statements.as_slice() {
            [Located {
                statement: JsStatement::Expression(expression),
                ..
            }] => fail_message(expression),
            _ => None,
        },
    }
}
