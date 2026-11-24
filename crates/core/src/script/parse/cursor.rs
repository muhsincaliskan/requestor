use serde_json::Value;

use super::lexer::Token;
use crate::script::{Operand, VariableTarget};

const TARGET_WORD: &str = "in";
const TARGETS: [(&str, VariableTarget); 3] = [
    ("environment", VariableTarget::Environment),
    ("collection", VariableTarget::Collection),
    ("request", VariableTarget::Request),
];
const LITERAL_WORDS: [(&str, Value); 3] = [
    ("true", Value::Bool(true)),
    ("false", Value::Bool(false)),
    ("null", Value::Null),
];

pub(super) struct Cursor {
    tokens: Vec<Token>,
    position: usize,
}

impl Cursor {
    pub(super) fn new(tokens: Vec<Token>) -> Self {
        Cursor {
            tokens,
            position: 0,
        }
    }

    pub(super) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    pub(super) fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();
        self.position += usize::from(token.is_some());
        token
    }

    pub(super) fn is_done(&self) -> bool {
        self.position >= self.tokens.len()
    }

    pub(super) fn at_word(&self, word: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(found)) if found.eq_ignore_ascii_case(word))
    }

    pub(super) fn at_words(&self, words: &[&str]) -> bool {
        words.iter().enumerate().all(|(offset, word)| {
            matches!(self.tokens.get(self.position + offset),
                Some(Token::Word(found)) if found.eq_ignore_ascii_case(word))
        })
    }

    pub(super) fn eat(&mut self, word: &str) -> bool {
        let found = self.at_word(word);
        self.position += usize::from(found);
        found
    }

    pub(super) fn eat_words(&mut self, words: &[&str]) -> bool {
        let found = self.at_words(words);
        if found {
            self.position += words.len();
        }
        found
    }

    pub(super) fn expect(&mut self, word: &str) -> Result<(), String> {
        if self.eat(word) {
            return Ok(());
        }
        Err(format!("expected \"{word}\"{}", self.found_text()))
    }

    pub(super) fn name(&mut self, what: &str) -> Result<String, String> {
        match self.next() {
            Some(Token::Word(name)) => Ok(name),
            Some(Token::Variable(name)) => Ok(name),
            _ => Err(format!("expected {what}")),
        }
    }

    pub(super) fn text(&mut self, what: &str) -> Result<String, String> {
        match self.next() {
            Some(Token::Text(text)) => Ok(text),
            _ => Err(format!("expected {what} in quotes")),
        }
    }

    pub(super) fn literal(&mut self) -> Option<Value> {
        let value = match self.peek()? {
            Token::Text(text) => Value::String(text.clone()),
            Token::Number(number) => Value::Number(number.clone()),
            Token::Word(word) => LITERAL_WORDS
                .iter()
                .find(|(name, _)| word.eq_ignore_ascii_case(name))?
                .1
                .clone(),
            Token::Variable(_) => return None,
        };
        self.position += 1;
        Some(value)
    }

    pub(super) fn operand(&mut self) -> Result<Operand, String> {
        if let Some(Token::Variable(name)) = self.peek() {
            let name = name.clone();
            self.position += 1;
            return Ok(Operand::Variable(name));
        }
        self.literal()
            .map(Operand::Literal)
            .ok_or_else(|| format!("expected a value{}", self.found_text()))
    }

    pub(super) fn target(&mut self) -> Result<VariableTarget, String> {
        if !self.eat(TARGET_WORD) {
            return Ok(VariableTarget::default());
        }
        let Some(Token::Word(word)) = self.next() else {
            return Err("expected environment, collection or request after \"in\"".to_string());
        };
        TARGETS
            .iter()
            .find(|(name, _)| word.eq_ignore_ascii_case(name))
            .map(|(_, target)| *target)
            .ok_or_else(|| {
                format!("unknown place \"{word}\": use environment, collection or request")
            })
    }

    pub(super) fn finish(&self) -> Result<(), String> {
        if self.is_done() {
            return Ok(());
        }
        Err(format!("unexpected{}", self.found_text()))
    }

    fn found_text(&self) -> String {
        match self.peek() {
            None => " at the end of the line".to_string(),
            Some(Token::Word(word)) => format!(", found \"{word}\""),
            Some(Token::Text(text)) => format!(", found \"{text}\" in quotes"),
            Some(Token::Number(number)) => format!(", found {number}"),
            Some(Token::Variable(name)) => format!(", found {{{{{name}}}}}"),
        }
    }
}

impl VariableTarget {
    pub fn word(self) -> &'static str {
        TARGETS
            .iter()
            .find(|(_, target)| *target == self)
            .map_or("", |(name, _)| name)
    }
}
