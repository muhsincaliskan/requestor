pub(crate) mod condition;
mod cursor;
mod lexer;
pub(crate) mod line;

use std::iter::Peekable;
use std::vec::IntoIter;

use line::{parse_line, Line};

use super::{Branch, Conditional, Phase, Script, Statement};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError {
    pub line: usize,
    pub message: String,
}

type Numbered = Peekable<IntoIter<(usize, Line)>>;

enum Closing {
    ElseIf(usize, super::Condition),
    Else(usize),
    End(usize),
    Finished,
}

impl Script {
    pub fn parse(text: &str, phase: Phase) -> Result<Script, Vec<ScriptError>> {
        let mut lines = Vec::new();
        let mut errors = Vec::new();
        for (index, line) in text.lines().enumerate() {
            match parse_line(line, phase) {
                Ok(parsed) => lines.push((index + 1, parsed)),
                Err(message) => errors.push(ScriptError {
                    line: index + 1,
                    message,
                }),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut numbered = lines.into_iter().peekable();
        let (statements, closing) = block(&mut numbered).map_err(|error| vec![error])?;
        match closing {
            Closing::Finished => Ok(Script { statements }.trimmed()),
            Closing::ElseIf(line, _) | Closing::Else(line) => Err(vec![stray(line, "else")]),
            Closing::End(line) => Err(vec![stray(line, "end")]),
        }
    }

    fn trimmed(mut self) -> Script {
        while matches!(self.statements.last(), Some(Statement::Blank)) {
            self.statements.pop();
        }
        self
    }
}

fn stray(line: usize, word: &str) -> ScriptError {
    ScriptError {
        line,
        message: format!("\"{word}\" has no matching \"if\""),
    }
}

fn block(lines: &mut Numbered) -> Result<(Vec<Statement>, Closing), ScriptError> {
    let mut statements = Vec::new();
    while let Some((number, line)) = lines.next() {
        match line {
            Line::Statement(statement) => statements.push(statement),
            Line::If(condition) => {
                statements.push(Statement::If(conditional(lines, number, condition)?))
            }
            Line::ElseIf(condition) => return Ok((statements, Closing::ElseIf(number, condition))),
            Line::Else => return Ok((statements, Closing::Else(number))),
            Line::End => return Ok((statements, Closing::End(number))),
        }
    }
    Ok((statements, Closing::Finished))
}

fn conditional(
    lines: &mut Numbered,
    opened: usize,
    condition: super::Condition,
) -> Result<Conditional, ScriptError> {
    let mut branches = Vec::new();
    let mut condition = condition;
    loop {
        let (body, closing) = block(lines)?;
        branches.push(Branch { condition, body });
        match closing {
            Closing::ElseIf(_, next) => condition = next,
            Closing::Else(_) => return otherwise(lines, opened, branches),
            Closing::End(_) => {
                return Ok(Conditional {
                    branches,
                    otherwise: None,
                })
            }
            Closing::Finished => return Err(unclosed(opened)),
        }
    }
}

fn otherwise(
    lines: &mut Numbered,
    opened: usize,
    branches: Vec<Branch>,
) -> Result<Conditional, ScriptError> {
    let (body, closing) = block(lines)?;
    match closing {
        Closing::End(_) => Ok(Conditional {
            branches,
            otherwise: Some(body),
        }),
        Closing::Finished => Err(unclosed(opened)),
        Closing::ElseIf(line, _) | Closing::Else(line) => Err(ScriptError {
            line,
            message: "\"else\" already used for this \"if\"; close it with \"end\"".to_string(),
        }),
    }
}

fn unclosed(line: usize) -> ScriptError {
    ScriptError {
        line,
        message: "this \"if\" has no \"end\"".to_string(),
    }
}

#[cfg(test)]
mod tests;
