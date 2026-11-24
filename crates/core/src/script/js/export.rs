use super::super::literal::quote;
use crate::script::dynamic::MILLIS_PER_SECOND;
use crate::script::{
    Comparison, Condition, Conditional, Dynamic, Operand, Script, SetValue, Statement, Subject,
    VariableTarget,
};

const INDENT: &str = "    ";

impl Script {
    pub fn javascript_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        write_block(&self.statements, 0, &mut lines);
        lines
    }
}

fn write_block(statements: &[Statement], depth: usize, lines: &mut Vec<String>) {
    for statement in statements {
        match statement {
            Statement::Blank => lines.push(String::new()),
            Statement::If(conditional) => write_conditional(conditional, depth, lines),
            other => lines.push(format!("{}{}", INDENT.repeat(depth), other.javascript())),
        }
    }
}

fn write_conditional(conditional: &Conditional, depth: usize, lines: &mut Vec<String>) {
    let indent = INDENT.repeat(depth);
    for (index, branch) in conditional.branches.iter().enumerate() {
        let opening = if index == 0 {
            format!("{indent}if")
        } else {
            format!("{indent}}} else if")
        };
        lines.push(format!("{opening} ({}) {{", branch.condition.javascript()));
        write_block(&branch.body, depth + 1, lines);
    }
    if let Some(otherwise) = &conditional.otherwise {
        lines.push(format!("{indent}}} else {{"));
        write_block(otherwise, depth + 1, lines);
    }
    lines.push(format!("{indent}}}"));
}

impl Statement {
    fn javascript(&self) -> String {
        match self {
            Statement::Blank | Statement::If(_) => String::new(),
            Statement::Comment(text) => format!("// {text}"),
            Statement::Set {
                variable,
                value,
                target,
            } => {
                format!(
                    "{}.set({}, {});",
                    target.javascript(),
                    quote(variable),
                    value.javascript()
                )
            }
            Statement::Clear { variable, target } => {
                format!("{}.unset({});", target.javascript(), quote(variable))
            }
            Statement::Save {
                subject,
                variable,
                target,
            } => {
                format!(
                    "{}.set({}, {});",
                    target.javascript(),
                    quote(variable),
                    subject.javascript()
                )
            }
            Statement::Expect { condition, name } => {
                let title = name.clone().unwrap_or_else(|| condition.title());
                format!(
                    "pm.test({}, () => {});",
                    quote(&title),
                    condition.assertion()
                )
            }
            Statement::Fail(message) => format!("throw new Error({});", quote(message)),
        }
    }
}

impl VariableTarget {
    pub(super) fn javascript(self) -> &'static str {
        match self {
            VariableTarget::Environment => "pm.environment",
            VariableTarget::Collection => "pm.collectionVariables",
            VariableTarget::Request => "pm.variables",
        }
    }
}

impl SetValue {
    fn javascript(&self) -> String {
        match self {
            SetValue::Literal(value) => value.to_string(),
            SetValue::Dynamic(dynamic) => dynamic.javascript(),
        }
    }
}

impl Dynamic {
    fn javascript(self) -> String {
        match self {
            Dynamic::Uuid => "pm.variables.replaceIn(\"{{$guid}}\")".to_string(),
            Dynamic::Timestamp => format!("Math.floor(Date.now() / {MILLIS_PER_SECOND})"),
            Dynamic::TimestampMillis => "Date.now()".to_string(),
            Dynamic::RandomNumber { min, max } => format!("_.random({min}, {max})"),
            Dynamic::NowIso => "new Date().toISOString()".to_string(),
        }
    }
}

impl Subject {
    fn javascript(&self) -> String {
        match self {
            Subject::Status => "pm.response.code".to_string(),
            Subject::Time => "pm.response.responseTime".to_string(),
            Subject::Body => "pm.response.text()".to_string(),
            Subject::Header(name) => format!("pm.response.headers.get({})", quote(name)),
            Subject::Json(path) => format!("pm.response.json(){}", path.accessor()),
            Subject::Variable(name) => format!("pm.variables.get({})", quote(name)),
        }
    }

    fn presence(&self) -> Option<String> {
        match self {
            Subject::Header(name) => Some(format!("pm.response.headers.has({})", quote(name))),
            Subject::Variable(name) => Some(format!("pm.variables.has({})", quote(name))),
            _ => None,
        }
    }
}

impl Operand {
    fn javascript(&self) -> String {
        match self {
            Operand::Literal(value) => value.to_string(),
            Operand::Variable(name) => format!("pm.variables.get({})", quote(name)),
        }
    }
}

impl Condition {
    fn javascript(&self) -> String {
        match self {
            Condition::Compare {
                subject,
                comparison,
                operand,
            } => expression(
                subject,
                *comparison,
                operand.as_ref().map(Operand::javascript),
            ),
            Condition::Not(inner) => format!("!({})", inner.javascript()),
            Condition::All(parts) => joined(parts, " && "),
            Condition::Any(parts) => joined(parts, " || "),
        }
    }

    fn assertion(&self) -> String {
        match self {
            Condition::Compare {
                subject,
                comparison,
                operand,
            } => chai(
                subject,
                *comparison,
                operand.as_ref().map(Operand::javascript),
            ),
            other => format!("pm.expect({}).to.be.true", other.javascript()),
        }
    }
}

fn joined(parts: &[Condition], separator: &str) -> String {
    parts
        .iter()
        .map(|part| match part {
            Condition::All(_) | Condition::Any(_) => format!("({})", part.javascript()),
            other => other.javascript(),
        })
        .collect::<Vec<_>>()
        .join(separator)
}

fn expression(subject: &Subject, comparison: Comparison, operand: Option<String>) -> String {
    let value = subject.javascript();
    let operand = operand.unwrap_or_default();
    match (comparison, subject.presence()) {
        (Comparison::Exists, Some(presence)) => presence,
        (Comparison::NotExists, Some(presence)) => format!("!{presence}"),
        (Comparison::Exists, None) => format!("{value} !== undefined"),
        (Comparison::NotExists, None) => format!("{value} === undefined"),
        (Comparison::Is, _) => format!("{value} == {operand}"),
        (Comparison::IsNot, _) => format!("{value} != {operand}"),
        (Comparison::Contains, _) => format!("{value}.includes({operand})"),
        (Comparison::NotContains, _) => format!("!{value}.includes({operand})"),
        (Comparison::Above, _) => format!("{value} > {operand}"),
        (Comparison::Below, _) => format!("{value} < {operand}"),
        (Comparison::IsEmpty, _) => format!("_.isEmpty({value})"),
        (Comparison::IsNotEmpty, _) => format!("!_.isEmpty({value})"),
    }
}

fn chai(subject: &Subject, comparison: Comparison, operand: Option<String>) -> String {
    let operand = operand.unwrap_or_default();
    match (subject, comparison) {
        (Subject::Status, Comparison::Is) if operand.parse::<u16>().is_ok() => {
            format!("pm.response.to.have.status({operand})")
        }
        (Subject::Header(name), Comparison::Exists) => {
            format!("pm.response.to.have.header({})", quote(name))
        }
        (Subject::Header(name), Comparison::NotExists) => {
            format!("pm.response.to.not.have.header({})", quote(name))
        }
        _ => format!(
            "pm.expect({}).to.{}",
            subject.javascript(),
            chain(comparison, &operand)
        ),
    }
}

fn chain(comparison: Comparison, operand: &str) -> String {
    match comparison {
        Comparison::Is => format!("eql({operand})"),
        Comparison::IsNot => format!("not.eql({operand})"),
        Comparison::Contains => format!("include({operand})"),
        Comparison::NotContains => format!("not.include({operand})"),
        Comparison::Above => format!("be.above({operand})"),
        Comparison::Below => format!("be.below({operand})"),
        Comparison::Exists => "not.be.undefined".to_string(),
        Comparison::NotExists => "be.undefined".to_string(),
        Comparison::IsEmpty => "be.empty".to_string(),
        Comparison::IsNotEmpty => "not.be.empty".to_string(),
    }
}
