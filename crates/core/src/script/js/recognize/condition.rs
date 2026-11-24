use super::super::syntax::{Body, Expr, JsStatement, Located};
use super::normalize::{self, expressible, normalized};
use super::Recognizer;
use crate::script::{Comparison, Condition, Operand, Subject};

const EXPECT_CALL: &str = "pm.expect";
const STATUS_ASSERTION: &str = "pm.response.to.have.status";
const HEADER_ASSERTION: &str = "pm.response.to.have.header";
const NO_HEADER_ASSERTION: &str = "pm.response.to.not.have.header";
const PRESENCE_CALLS: [&str; 2] = ["pm.response.headers.has", "pm.variables.has"];
const EMPTY_CALL: &str = "_.isEmpty";
const INCLUDES: &str = "includes";
const UNDEFINED: &str = "undefined";
const FILLER_WORDS: [&str; 7] = ["to", "be", "have", "a", "an", "deep", "that"];
const NOT: &str = "not";

impl Recognizer {
    pub(super) fn condition(&self, expression: &Expr) -> Option<Condition> {
        expressible(normalized(self.raw_condition(expression)?))
    }

    fn raw_condition(&self, expression: &Expr) -> Option<Condition> {
        match expression {
            Expr::Group(inner) => self.raw_condition(inner),
            Expr::Binary("&&", left, right) => Some(Condition::All(vec![
                self.raw_condition(left)?,
                self.raw_condition(right)?,
            ])),
            Expr::Binary("||", left, right) => Some(Condition::Any(vec![
                self.raw_condition(left)?,
                self.raw_condition(right)?,
            ])),
            Expr::Not(inner) => self.negation(inner),
            Expr::Binary(operator, left, right) => self.binary(operator, left, right),
            other => self.call_condition(other),
        }
    }

    fn negation(&self, inner: &Expr) -> Option<Condition> {
        if !matches!(inner, Expr::Group(_)) {
            if let Some(condition) = self.call_condition(inner) {
                return Some(normalize::negated(condition));
            }
        }
        Some(Condition::Not(Box::new(self.raw_condition(inner)?)))
    }

    fn binary(&self, operator: &str, left: &Expr, right: &Expr) -> Option<Condition> {
        let subject = self.subject(left)?;
        if right.path().as_deref() == Some(UNDEFINED) {
            let comparison = match operator {
                "===" | "==" => Comparison::NotExists,
                "!==" | "!=" => Comparison::Exists,
                _ => return None,
            };
            return Some(compare(subject, comparison, None));
        }
        let comparison = match operator {
            "===" | "==" => Comparison::Is,
            "!==" | "!=" => Comparison::IsNot,
            ">" => Comparison::Above,
            "<" => Comparison::Below,
            _ => return None,
        };
        let operand = self.operand_for(&subject, right)?;
        Some(compare(subject, comparison, Some(operand)))
    }

    fn call_condition(&self, expression: &Expr) -> Option<Condition> {
        if let Some((object, [needle])) = expression.method_call(INCLUDES) {
            return Some(compare(
                self.subject(object)?,
                Comparison::Contains,
                Some(self.operand(needle)?),
            ));
        }
        if let Some([value]) = expression.call_of(EMPTY_CALL) {
            return Some(compare(self.subject(value)?, Comparison::IsEmpty, None));
        }
        let Some(presence) = PRESENCE_CALLS
            .iter()
            .find_map(|call| expression.call_of(call))
        else {
            return self.truthy(expression);
        };
        let [Expr::Text(name)] = presence else {
            return None;
        };
        let subject = match expression.call_of(PRESENCE_CALLS[0]) {
            Some(_) => Subject::Header(name.clone()),
            None => Subject::Variable(name.clone()),
        };
        Some(compare(subject, Comparison::Exists, None))
    }

    fn truthy(&self, expression: &Expr) -> Option<Condition> {
        match self.subject(expression)? {
            subject @ Subject::Json(_) => Some(compare(subject, Comparison::Exists, None)),
            _ => None,
        }
    }

    pub(super) fn test_condition(&self, body: &Body) -> Option<Condition> {
        let assertions: Vec<&Expr> = match body {
            Body::Expression(expression) => vec![expression],
            Body::Block(statements) => assertion_statements(statements)?,
        };
        let mut conditions = assertions
            .into_iter()
            .map(|assertion| self.assertion(assertion))
            .collect::<Option<Vec<_>>>()?;
        match conditions.len() {
            0 => None,
            1 => conditions.pop(),
            _ => expressible(normalized(Condition::All(conditions))),
        }
    }

    fn assertion(&self, expression: &Expr) -> Option<Condition> {
        if let Some([code]) = expression.call_of(STATUS_ASSERTION) {
            return Some(compare(
                Subject::Status,
                Comparison::Is,
                Some(self.operand_for(&Subject::Status, code)?),
            ));
        }
        for (call, comparison) in [
            (HEADER_ASSERTION, Comparison::Exists),
            (NO_HEADER_ASSERTION, Comparison::NotExists),
        ] {
            if let Some([Expr::Text(name)]) = expression.call_of(call) {
                return Some(compare(Subject::Header(name.clone()), comparison, None));
            }
        }
        let (value, words, argument) = chai_chain(expression)?;
        self.chai(value, &words, argument)
    }

    fn chai(&self, value: &Expr, words: &[String], argument: Option<&Expr>) -> Option<Condition> {
        let negated = words.iter().filter(|word| *word == NOT).count() % 2 == 1;
        let meaningful: Vec<&str> = words
            .iter()
            .map(String::as_str)
            .filter(|word| *word != NOT && !FILLER_WORDS.contains(word))
            .collect();
        let [word] = meaningful.as_slice() else {
            return None;
        };
        if *word == "true" || *word == "false" {
            let condition = self.condition(value)?;
            let flipped = negated != (*word == "false");
            return Some(if flipped {
                Condition::Not(Box::new(condition))
            } else {
                condition
            });
        }
        let (comparison, needs_argument) = chai_word(word)?;
        let subject = self.subject(value)?;
        let operand = match (needs_argument, argument) {
            (true, Some(argument)) => Some(self.operand_for(&subject, argument)?),
            (false, None) => None,
            _ => return None,
        };
        let condition = compare(subject, comparison, operand);
        Some(if negated {
            normalize::negated(condition)
        } else {
            condition
        })
    }
}

fn chai_word(word: &str) -> Option<(Comparison, bool)> {
    Some(match word {
        "eql" | "equal" | "equals" | "eq" => (Comparison::Is, true),
        "include" | "includes" | "contain" | "contains" => (Comparison::Contains, true),
        "above" | "greaterThan" | "gt" => (Comparison::Above, true),
        "below" | "lessThan" | "lt" => (Comparison::Below, true),
        "undefined" => (Comparison::NotExists, false),
        "exist" => (Comparison::Exists, false),
        "empty" => (Comparison::IsEmpty, false),
        _ => return None,
    })
}

fn chai_chain(expression: &Expr) -> Option<(&Expr, Vec<String>, Option<&Expr>)> {
    let (mut current, argument) = match expression {
        Expr::Call(callee, arguments) if arguments.len() <= 1 => {
            (callee.as_ref(), arguments.first())
        }
        other => (other, None),
    };
    let mut words = Vec::new();
    loop {
        match current {
            Expr::Member(object, word) => {
                words.push(word.clone());
                current = object;
            }
            Expr::Call(_, _) => {
                let [value] = current.call_of(EXPECT_CALL)? else {
                    return None;
                };
                words.reverse();
                return Some((value, words, argument));
            }
            _ => return None,
        }
    }
}

fn assertion_statements(statements: &[Located]) -> Option<Vec<&Expr>> {
    statements
        .iter()
        .filter(|located| {
            !matches!(
                located.statement,
                JsStatement::Blank | JsStatement::Comment(_)
            )
        })
        .map(|located| match &located.statement {
            JsStatement::Expression(expression) => Some(expression),
            _ => None,
        })
        .collect()
}

pub(super) fn compare(
    subject: Subject,
    comparison: Comparison,
    operand: Option<Operand>,
) -> Condition {
    Condition::Compare {
        subject,
        comparison,
        operand,
    }
}
