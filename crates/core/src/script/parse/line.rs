use super::condition::ConditionParser;
use super::cursor::Cursor;
use super::lexer::{tokens, Token};
use crate::script::{Condition, Dynamic, Phase, SetValue, Statement};

const COMMENT_MARK: &str = "#";
pub(crate) const COMMANDS: [&str; 9] = [
    "set", "clear", "expect", "save", "fail", "if", "else", "end", "#",
];

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Line {
    Statement(Statement),
    If(Condition),
    ElseIf(Condition),
    Else,
    End,
}

pub(super) fn parse_line(text: &str, phase: Phase) -> Result<Line, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Line::Statement(Statement::Blank));
    }
    if let Some(comment) = trimmed.strip_prefix(COMMENT_MARK) {
        return Ok(Line::Statement(Statement::Comment(
            comment.trim().to_string(),
        )));
    }
    let mut cursor = Cursor::new(tokens(trimmed)?);
    let Some(Token::Word(command)) = cursor.next() else {
        return Err("a line starts with a command such as set, expect or save".to_string());
    };
    let line = LineParser {
        cursor: &mut cursor,
        phase,
    }
    .command(&command.to_ascii_lowercase())?;
    cursor.finish()?;
    Ok(line)
}

struct LineParser<'a> {
    cursor: &'a mut Cursor,
    phase: Phase,
}

impl LineParser<'_> {
    fn command(&mut self, command: &str) -> Result<Line, String> {
        match command {
            "set" => self.set().map(Line::Statement),
            "clear" => self.clear().map(Line::Statement),
            "expect" => self.expect().map(Line::Statement),
            "save" => self.save().map(Line::Statement),
            "fail" => self
                .cursor
                .text("a message")
                .map(|message| Line::Statement(Statement::Fail(message))),
            "if" => self.condition().map(Line::If),
            "else" if self.cursor.eat("if") => self.condition().map(Line::ElseIf),
            "else" => Ok(Line::Else),
            "end" => Ok(Line::End),
            other => Err(unknown_command(other)),
        }
    }

    fn condition(&mut self) -> Result<Condition, String> {
        ConditionParser {
            cursor: self.cursor,
            phase: self.phase,
        }
        .condition()
    }

    fn set(&mut self) -> Result<Statement, String> {
        let variable = self.cursor.name("a variable name")?;
        self.cursor.expect("to")?;
        let value = self.set_value()?;
        Ok(Statement::Set {
            variable,
            value,
            target: self.cursor.target()?,
        })
    }

    fn set_value(&mut self) -> Result<SetValue, String> {
        if let Some(Token::Variable(name)) = self.cursor.peek() {
            let text = format!("{{{{{name}}}}}");
            self.cursor.next();
            return Ok(SetValue::Literal(serde_json::Value::String(text)));
        }
        if let Some(literal) = self.cursor.literal() {
            return Ok(SetValue::Literal(literal));
        }
        self.dynamic().map(SetValue::Dynamic)
    }

    fn dynamic(&mut self) -> Result<Dynamic, String> {
        if self.cursor.eat_words(&["new", "uuid"]) {
            return Ok(Dynamic::Uuid);
        }
        if self.cursor.eat_words(&["now", "iso"]) {
            return Ok(Dynamic::NowIso);
        }
        if self.cursor.eat("timestamp") {
            let millis = self.cursor.eat("ms");
            return Ok(if millis {
                Dynamic::TimestampMillis
            } else {
                Dynamic::Timestamp
            });
        }
        if self.cursor.eat_words(&["random", "number"]) {
            let min = self.integer()?;
            let max = self.integer()?;
            return Ok(Dynamic::RandomNumber { min, max });
        }
        Err("expected a value: text in quotes, a number, true, false, new uuid, timestamp, random number or now iso".to_string())
    }

    fn integer(&mut self) -> Result<i64, String> {
        match self.cursor.next() {
            Some(Token::Number(number)) => number
                .as_i64()
                .ok_or_else(|| "expected a whole number".to_string()),
            _ => Err("random number needs two whole numbers, like random number 1 100".to_string()),
        }
    }

    fn clear(&mut self) -> Result<Statement, String> {
        Ok(Statement::Clear {
            variable: self.cursor.name("a variable name")?,
            target: self.cursor.target()?,
        })
    }

    fn expect(&mut self) -> Result<Statement, String> {
        let condition = self.condition()?;
        let name = if self.cursor.eat("named") {
            Some(self.cursor.text("a test name")?)
        } else {
            None
        };
        Ok(Statement::Expect { condition, name })
    }

    fn save(&mut self) -> Result<Statement, String> {
        let subject = ConditionParser {
            cursor: self.cursor,
            phase: self.phase,
        }
        .subject()?;
        self.cursor.expect("as")?;
        Ok(Statement::Save {
            subject,
            variable: self.cursor.name("a variable name")?,
            target: self.cursor.target()?,
        })
    }
}

fn unknown_command(command: &str) -> String {
    match closest_command(command) {
        Some(suggestion) => {
            format!("unknown command \"{command}\" (did you mean \"{suggestion}\"?)")
        }
        None => format!("unknown command \"{command}\": use set, clear, expect, save, fail or if"),
    }
}

fn closest_command(command: &str) -> Option<&'static str> {
    const MAX_DISTANCE: usize = 2;
    if let Some(started) = COMMANDS
        .iter()
        .find(|candidate| candidate.starts_with(command))
    {
        return Some(started);
    }
    COMMANDS
        .iter()
        .map(|candidate| (distance(command, candidate), *candidate))
        .filter(|(found, _)| *found <= MAX_DISTANCE)
        .min_by_key(|(found, _)| *found)
        .map(|(_, candidate)| candidate)
}

fn distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (row, left_char) in left.chars().enumerate() {
        let mut current = vec![row + 1];
        for (column, right_char) in right.iter().enumerate() {
            let substitution = previous[column] + usize::from(left_char != *right_char);
            current.push(
                substitution
                    .min(previous[column + 1] + 1)
                    .min(current[column] + 1),
            );
        }
        previous = current;
    }
    previous[right.len()]
}
