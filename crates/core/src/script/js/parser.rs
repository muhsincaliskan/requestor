use std::collections::HashSet;

mod cursor;
mod expression;

use super::lexer::{Kind, Token};
use super::syntax::{JsStatement, Located};

const DECLARATIONS: [&str; 3] = ["var", "let", "const"];
const LEVELS: [&[&str]; 6] = [
    &["||"],
    &["&&"],
    &["===", "!==", "==", "!="],
    &["<", ">", "<=", ">="],
    &["+", "-"],
    &["*", "/"],
];

pub(super) type Failure = (usize, String);

pub(super) struct Parser {
    tokens: Vec<Token>,
    position: usize,
    blank_lines: HashSet<usize>,
}

impl Parser {
    pub(super) fn new(tokens: Vec<Token>, source: &str) -> Self {
        let blank_lines = source
            .lines()
            .enumerate()
            .filter(|(_, line)| line.trim().is_empty())
            .map(|(index, _)| index + 1)
            .collect();
        Parser {
            tokens,
            position: 0,
            blank_lines,
        }
    }

    pub(super) fn program(&mut self) -> Result<Vec<Located>, Failure> {
        let statements = self.statements_until(None)?;
        Ok(statements)
    }

    fn statements_until(&mut self, closing: Option<&str>) -> Result<Vec<Located>, Failure> {
        let mut found: Vec<Located> = Vec::new();
        loop {
            if closing.is_some_and(|mark| self.eat(mark)) {
                return Ok(found);
            }
            let Some(token) = self.peek() else {
                return match closing {
                    None => Ok(found),
                    Some(mark) => Err(self.failure(&format!("missing \"{mark}\""))),
                };
            };
            let first_line = token.line;
            if self.eat(";") {
                continue;
            }
            let statement = self.statement()?;
            let last_line = self.previous_line();
            self.add_blank_lines(&mut found, first_line);
            found.push(Located {
                statement,
                first_line,
                last_line,
            });
        }
    }

    fn add_blank_lines(&self, found: &mut Vec<Located>, next_line: usize) {
        let Some(previous) = found.last().map(|located| located.last_line) else {
            return;
        };
        for line in previous + 1..next_line {
            if self.blank_lines.contains(&line) {
                found.push(Located {
                    statement: JsStatement::Blank,
                    first_line: line,
                    last_line: line,
                });
            }
        }
    }

    fn statement(&mut self) -> Result<JsStatement, Failure> {
        if let Some(Kind::Comment(text)) = self.peek().map(|token| token.kind.clone()) {
            self.position += 1;
            return Ok(JsStatement::Comment(text));
        }
        if self.eat_name("if") {
            return self.if_statement();
        }
        if self.eat_name("throw") {
            let value = self.expression()?;
            self.eat(";");
            return Ok(JsStatement::Throw(value));
        }
        if DECLARATIONS.iter().any(|word| self.eat_name(word)) {
            return self.declaration();
        }
        let value = self.expression()?;
        self.eat(";");
        Ok(JsStatement::Expression(value))
    }

    fn declaration(&mut self) -> Result<JsStatement, Failure> {
        let name = self.name()?;
        self.expect("=")?;
        let value = self.expression()?;
        self.eat(";");
        Ok(JsStatement::Declare { name, value })
    }

    fn if_statement(&mut self) -> Result<JsStatement, Failure> {
        self.expect("(")?;
        let condition = self.expression()?;
        self.expect(")")?;
        let then = self.block()?;
        let otherwise = if !self.eat_name("else") {
            None
        } else if self.eat_name("if") {
            let line = self.previous_line();
            let nested = self.if_statement()?;
            Some(vec![Located {
                statement: nested,
                first_line: line,
                last_line: self.previous_line(),
            }])
        } else {
            Some(self.block()?)
        };
        Ok(JsStatement::If {
            condition,
            then,
            otherwise,
        })
    }

    fn block(&mut self) -> Result<Vec<Located>, Failure> {
        self.expect("{")?;
        self.statements_until(Some("}"))
    }
}
