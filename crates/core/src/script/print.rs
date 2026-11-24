use serde_json::Value;

use super::{
    Comparison, Condition, Conditional, Dynamic, Operand, Script, SetValue, Statement, Subject,
    VariableTarget,
};

const INDENT: &str = "  ";
const MILLISECONDS: &str = " ms";

impl Script {
    pub fn text(&self) -> String {
        let mut lines = Vec::new();
        write_block(&self.statements, 0, &mut lines);
        lines.join("\n")
    }
}

fn write_block(statements: &[Statement], depth: usize, lines: &mut Vec<String>) {
    for statement in statements {
        match statement {
            Statement::If(conditional) => write_conditional(conditional, depth, lines),
            Statement::Blank => lines.push(String::new()),
            other => lines.push(format!("{}{}", INDENT.repeat(depth), other.line_text())),
        }
    }
}

fn write_conditional(conditional: &Conditional, depth: usize, lines: &mut Vec<String>) {
    let indent = INDENT.repeat(depth);
    for (index, branch) in conditional.branches.iter().enumerate() {
        let keyword = if index == 0 { "if" } else { "else if" };
        lines.push(format!("{indent}{keyword} {}", branch.condition.text()));
        write_block(&branch.body, depth + 1, lines);
    }
    if let Some(otherwise) = &conditional.otherwise {
        lines.push(format!("{indent}else"));
        write_block(otherwise, depth + 1, lines);
    }
    lines.push(format!("{indent}end"));
}

impl Statement {
    fn line_text(&self) -> String {
        match self {
            Statement::Blank | Statement::If(_) => String::new(),
            Statement::Comment(text) => format!("# {text}"),
            Statement::Set {
                variable,
                value,
                target,
            } => {
                format!(
                    "set {variable} to {}{}",
                    value.text(),
                    target_suffix(*target)
                )
            }
            Statement::Clear { variable, target } => {
                format!("clear {variable}{}", target_suffix(*target))
            }
            Statement::Expect { condition, name } => match name {
                Some(name) => format!("expect {} named {}", condition.text(), quoted(name)),
                None => format!("expect {}", condition.text()),
            },
            Statement::Save {
                subject,
                variable,
                target,
            } => {
                format!(
                    "save {} as {variable}{}",
                    subject.text(),
                    target_suffix(*target)
                )
            }
            Statement::Fail(message) => format!("fail {}", quoted(message)),
        }
    }
}

fn target_suffix(target: VariableTarget) -> String {
    if target == VariableTarget::default() {
        return String::new();
    }
    format!(" in {}", target.word())
}

fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_default()
}

impl SetValue {
    fn text(&self) -> String {
        match self {
            SetValue::Literal(value) => value.to_string(),
            SetValue::Dynamic(dynamic) => dynamic.text(),
        }
    }
}

impl Dynamic {
    pub fn text(self) -> String {
        match self {
            Dynamic::Uuid => "new uuid".to_string(),
            Dynamic::Timestamp => "timestamp".to_string(),
            Dynamic::TimestampMillis => "timestamp ms".to_string(),
            Dynamic::RandomNumber { min, max } => format!("random number {min} {max}"),
            Dynamic::NowIso => "now iso".to_string(),
        }
    }
}

impl Condition {
    pub fn text(&self) -> String {
        match self {
            Condition::Compare {
                subject,
                comparison,
                operand,
            } => comparison_text(subject, *comparison, operand.as_ref()),
            Condition::Not(inner) => format!("not {}", inner.text()),
            Condition::All(parts) => joined(parts, " and "),
            Condition::Any(parts) => joined(parts, " or "),
        }
    }
}

fn joined(parts: &[Condition], separator: &str) -> String {
    parts
        .iter()
        .map(Condition::text)
        .collect::<Vec<_>>()
        .join(separator)
}

fn comparison_text(subject: &Subject, comparison: Comparison, operand: Option<&Operand>) -> String {
    let mut text = format!("{} {}", subject.text(), comparison.words());
    if let Some(operand) = operand {
        text.push(' ');
        text.push_str(&operand.text());
        if *subject == Subject::Time && matches!(operand, Operand::Literal(Value::Number(_))) {
            text.push_str(MILLISECONDS);
        }
    }
    text
}

impl Operand {
    pub fn text(&self) -> String {
        match self {
            Operand::Literal(value) => value.to_string(),
            Operand::Variable(name) => format!("{{{{{name}}}}}"),
        }
    }
}

impl Subject {
    pub fn text(&self) -> String {
        match self {
            Subject::Header(name) => format!("header {name}"),
            Subject::Json(path) if path.is_root() => "json".to_string(),
            Subject::Json(path) => format!("json {}", path.text()),
            Subject::Variable(name) => format!("{{{{{name}}}}}"),
            other => other.keyword().to_string(),
        }
    }
}
