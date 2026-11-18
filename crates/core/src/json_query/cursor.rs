use super::InvalidQuery;
use crate::json_tree::JsonNode;

const NAME_END: [char; 4] = ['.', '[', ']', ','];
const SINGLE_QUOTE: char = '\'';
const DOUBLE_QUOTE: char = '"';
const ESCAPE: char = '\\';
const TRUE: &str = "true";
const FALSE: &str = "false";
const NULL: &str = "null";

pub(super) struct Cursor<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> Cursor<'a> {
    pub(super) fn new(text: &'a str) -> Self {
        Cursor { text, position: 0 }
    }

    pub(super) fn rest(&self) -> &'a str {
        &self.text[self.position..]
    }

    pub(super) fn is_done(&self) -> bool {
        self.rest().is_empty()
    }

    pub(super) fn invalid(&self) -> InvalidQuery {
        InvalidQuery {
            position: self.text[..self.position].chars().count() + 1,
        }
    }

    pub(super) fn eat(&mut self, expected: &str) -> bool {
        let found = self.rest().starts_with(expected);
        if found {
            self.position += expected.len();
        }
        found
    }

    pub(super) fn expect(&mut self, expected: &str) -> Result<(), InvalidQuery> {
        self.skip_space();
        if self.eat(expected) {
            Ok(())
        } else {
            Err(self.invalid())
        }
    }

    pub(super) fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    pub(super) fn skip_space(&mut self) {
        let rest = self.rest();
        self.position += rest.len() - rest.trim_start().len();
    }

    pub(super) fn name(&mut self) -> Result<String, InvalidQuery> {
        let rest = self.rest();
        let end = rest
            .find(|character: char| NAME_END.contains(&character) || character.is_whitespace())
            .unwrap_or(rest.len());
        if end == 0 {
            return Err(self.invalid());
        }
        self.position += end;
        Ok(rest[..end].to_string())
    }

    pub(super) fn starts_quoted(&self) -> bool {
        matches!(self.peek(), Some(SINGLE_QUOTE | DOUBLE_QUOTE))
    }

    pub(super) fn quoted(&mut self) -> Result<String, InvalidQuery> {
        let quote = self.peek().ok_or_else(|| self.invalid())?;
        let mut text = String::new();
        let mut escaped = false;
        for (offset, character) in self.rest().char_indices().skip(1) {
            if !escaped && character == quote {
                self.position += offset + character.len_utf8();
                return Ok(text);
            }
            escaped = !escaped && character == ESCAPE;
            if !escaped {
                text.push(character);
            }
        }
        Err(self.invalid())
    }

    pub(super) fn integer(&mut self) -> Result<Option<i64>, InvalidQuery> {
        self.skip_space();
        let rest = self.rest();
        let sign = usize::from(rest.starts_with('-'));
        let digits = rest[sign..]
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(rest.len() - sign);
        if digits == 0 {
            return Ok(None);
        }
        let number = rest[..sign + digits].parse().map_err(|_| self.invalid())?;
        self.position += sign + digits;
        Ok(Some(number))
    }

    pub(super) fn literal(&mut self) -> Result<JsonNode, InvalidQuery> {
        self.skip_space();
        if self.starts_quoted() {
            return self.quoted().map(JsonNode::String);
        }
        for (word, node) in [
            (TRUE, JsonNode::Bool(true)),
            (FALSE, JsonNode::Bool(false)),
            (NULL, JsonNode::Null),
        ] {
            if self.eat(word) {
                return Ok(node);
            }
        }
        self.number()
    }

    fn number(&mut self) -> Result<JsonNode, InvalidQuery> {
        let rest = self.rest();
        let end = rest
            .find(|character: char| !(character.is_ascii_digit() || "-+.eE".contains(character)))
            .unwrap_or(rest.len());
        let node = JsonNode::parse(&rest[..end]).filter(|node| node.as_number().is_some());
        let node = node.ok_or_else(|| self.invalid())?;
        self.position += end;
        Ok(node)
    }
}
