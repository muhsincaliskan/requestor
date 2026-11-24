use super::cursor::Cursor;
use super::lexer::Token;
use crate::script::{Comparison, Condition, JsonPath, Phase, Subject};

const AND: &str = "and";
const OR: &str = "or";
const NOT: &str = "not";
const MILLISECONDS: &str = "ms";
const SAVE_SEPARATOR: &str = "as";
pub(crate) const COMPARISONS: [(&[&str], Comparison); 10] = [
    (&["is", "not", "empty"], Comparison::IsNotEmpty),
    (&["is", "empty"], Comparison::IsEmpty),
    (&["is", "not"], Comparison::IsNot),
    (&["is"], Comparison::Is),
    (&["not", "contains"], Comparison::NotContains),
    (&["contains"], Comparison::Contains),
    (&["above"], Comparison::Above),
    (&["below"], Comparison::Below),
    (&["not", "exists"], Comparison::NotExists),
    (&["exists"], Comparison::Exists),
];
pub(crate) const SUBJECT_WORDS: [&str; 5] = ["status", "time", "body", "header", "json"];

pub(super) struct ConditionParser<'a> {
    pub(super) cursor: &'a mut Cursor,
    pub(super) phase: Phase,
}

impl ConditionParser<'_> {
    pub(super) fn condition(&mut self) -> Result<Condition, String> {
        let mut any = vec![self.all()?];
        while self.cursor.eat(OR) {
            any.push(self.all()?);
        }
        Ok(collapse(any, Condition::Any))
    }

    fn all(&mut self) -> Result<Condition, String> {
        let mut all = vec![self.unary()?];
        while self.cursor.eat(AND) {
            all.push(self.unary()?);
        }
        Ok(collapse(all, Condition::All))
    }

    fn unary(&mut self) -> Result<Condition, String> {
        if self.cursor.eat(NOT) {
            return Ok(Condition::Not(Box::new(self.unary()?)));
        }
        self.comparison()
    }

    fn comparison(&mut self) -> Result<Condition, String> {
        let subject = self.subject()?;
        let comparison = COMPARISONS
            .iter()
            .find(|(words, _)| self.cursor.eat_words(words))
            .map(|(_, comparison)| *comparison)
            .ok_or("expected is, is not, contains, above, below, exists or is empty")?;
        let operand = if comparison.takes_operand() {
            let operand = self.cursor.operand()?;
            self.cursor.eat(MILLISECONDS);
            Some(operand)
        } else {
            None
        };
        Ok(Condition::Compare {
            subject,
            comparison,
            operand,
        })
    }

    pub(super) fn subject(&mut self) -> Result<Subject, String> {
        let subject = subject(self.cursor)?;
        if self.phase == Phase::BeforeRequest && subject.needs_response() {
            return Err(format!(
                "\"{}\" needs the response; use it in After response",
                subject.keyword()
            ));
        }
        Ok(subject)
    }
}

fn subject(cursor: &mut Cursor) -> Result<Subject, String> {
    match cursor.next() {
        Some(Token::Variable(name)) => Ok(Subject::Variable(name)),
        Some(Token::Word(word)) => word_subject(cursor, &word.to_ascii_lowercase()),
        _ => Err("expected status, time, body, header, json or a {{variable}}".to_string()),
    }
}

fn word_subject(cursor: &mut Cursor, word: &str) -> Result<Subject, String> {
    match word {
        "status" => Ok(Subject::Status),
        "time" => Ok(Subject::Time),
        "body" => Ok(Subject::Body),
        "header" => cursor.name("a header name").map(Subject::Header),
        "json" => json_subject(cursor),
        other => Err(format!(
            "unknown value \"{other}\": use status, time, body, header, json or a {{{{variable}}}}"
        )),
    }
}

fn json_subject(cursor: &mut Cursor) -> Result<Subject, String> {
    let path_follows = matches!(cursor.peek(), Some(Token::Word(_)))
        && !starts_comparison(cursor)
        && !cursor.at_word(SAVE_SEPARATOR);
    if !path_follows {
        return Ok(Subject::Json(JsonPath::default()));
    }
    let Some(Token::Word(text)) = cursor.next() else {
        unreachable!("checked above");
    };
    JsonPath::parse(&text)
        .map(Subject::Json)
        .ok_or_else(|| format!("\"{text}\" is not a JSON path like data.items[0].id"))
}

fn starts_comparison(cursor: &Cursor) -> bool {
    COMPARISONS
        .iter()
        .any(|(words, _)| cursor.at_word(words[0]))
}

fn collapse(mut conditions: Vec<Condition>, join: fn(Vec<Condition>) -> Condition) -> Condition {
    if conditions.len() == 1 {
        return conditions.remove(0);
    }
    join(conditions)
}

impl Subject {
    pub(crate) fn keyword(&self) -> &'static str {
        match self {
            Subject::Status => "status",
            Subject::Time => "time",
            Subject::Body => "body",
            Subject::Header(_) => "header",
            Subject::Json(_) => "json",
            Subject::Variable(_) => "variable",
        }
    }
}

impl Comparison {
    pub(crate) fn words(self) -> String {
        COMPARISONS
            .iter()
            .find(|(_, comparison)| *comparison == self)
            .map(|(words, _)| words.join(" "))
            .unwrap_or_default()
    }
}
