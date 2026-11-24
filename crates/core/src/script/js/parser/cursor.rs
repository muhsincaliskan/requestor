use super::super::lexer::{Kind, Token};
use super::{Failure, Parser};

impl Parser {
    pub(super) fn name(&mut self) -> Result<String, Failure> {
        match self.next() {
            Some(Token {
                kind: Kind::Name(name),
                ..
            }) => Ok(name),
            _ => Err(self.failure("expected a name")),
        }
    }

    pub(super) fn expect(&mut self, mark: &str) -> Result<(), Failure> {
        if self.eat(mark) {
            return Ok(());
        }
        Err(self.failure(&format!("expected \"{mark}\"")))
    }

    pub(super) fn at(&self, mark: &str) -> bool {
        matches!(self.peek(), Some(Token { kind: Kind::Punct(found), .. }) if *found == mark)
    }

    pub(super) fn eat(&mut self, mark: &str) -> bool {
        let found = self.at(mark);
        self.position += usize::from(found);
        found
    }

    pub(super) fn eat_name(&mut self, word: &str) -> bool {
        let found =
            matches!(self.peek(), Some(Token { kind: Kind::Name(name), .. }) if name == word);
        self.position += usize::from(found);
        found
    }

    pub(super) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    pub(super) fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();
        self.position += usize::from(token.is_some());
        token
    }

    pub(super) fn previous_line(&self) -> usize {
        self.position
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map_or(1, |token| token.line)
    }

    pub(super) fn failure(&self, message: &str) -> Failure {
        let line = self
            .peek()
            .map_or_else(|| self.previous_line(), |token| token.line);
        (line, message.to_string())
    }
}
