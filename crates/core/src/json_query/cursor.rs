use super::InvalidQuery;
use crate::json_tree::JsonNode;

const NAME_PUNCTUATION: [char; 2] = ['_', '-'];
const MINUS: char = '-';
const NUMBER_CHARACTERS: &str = "-+.eE";
const UNICODE_ESCAPE: char = 'u';
const UNICODE_DIGITS: usize = 4;
const HEX_RADIX: u32 = 16;
const ESCAPES: [(char, char); 8] = [
    ('n', '\n'),
    ('t', '\t'),
    ('r', '\r'),
    ('b', '\u{8}'),
    ('f', '\u{c}'),
    ('/', '/'),
    ('\\', '\\'),
    ('\'', '\''),
];
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
            .find(|character: char| !is_name_character(character))
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
        let rest = self.rest();
        let mut escaped = false;
        let close = rest.char_indices().skip(1).find(|(_, character)| {
            let closes = !escaped && *character == quote;
            escaped = !escaped && *character == ESCAPE;
            closes
        });
        let (close, _) = close.ok_or_else(|| self.invalid())?;
        let text = unescaped(&rest[quote.len_utf8()..close]).ok_or_else(|| self.invalid())?;
        self.position += close + quote.len_utf8();
        Ok(text)
    }

    pub(super) fn integer(&mut self) -> Result<Option<i64>, InvalidQuery> {
        self.skip_space();
        let rest = self.rest();
        let sign = usize::from(rest.starts_with(MINUS));
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
            if self.eat_word(word) {
                return Ok(node);
            }
        }
        self.number()
    }

    fn eat_word(&mut self, word: &str) -> bool {
        let rest = self.rest();
        let ends = rest
            .strip_prefix(word)
            .is_some_and(|after| !after.starts_with(is_name_character));
        if ends {
            self.position += word.len();
        }
        ends
    }

    fn number(&mut self) -> Result<JsonNode, InvalidQuery> {
        let rest = self.rest();
        let end = rest
            .find(|character: char| {
                !(character.is_ascii_digit() || NUMBER_CHARACTERS.contains(character))
            })
            .unwrap_or(rest.len());
        let node = JsonNode::parse(&rest[..end]).filter(|node| node.as_number().is_some());
        let node = node.ok_or_else(|| self.invalid())?;
        self.position += end;
        Ok(node)
    }
}

fn is_name_character(character: char) -> bool {
    character.is_alphanumeric() || NAME_PUNCTUATION.contains(&character) || !character.is_ascii()
}

fn unescaped(raw: &str) -> Option<String> {
    let mut text = String::with_capacity(raw.len());
    let mut characters = raw.chars();
    while let Some(character) = characters.next() {
        if character != ESCAPE {
            text.push(character);
            continue;
        }
        text.push(escape_target(&mut characters)?);
    }
    Some(text)
}

fn escape_target(characters: &mut std::str::Chars) -> Option<char> {
    let marker = characters.next()?;
    if marker == UNICODE_ESCAPE {
        let digits: String = characters.by_ref().take(UNICODE_DIGITS).collect();
        return char::from_u32(u32::from_str_radix(&digits, HEX_RADIX).ok()?);
    }
    if marker == DOUBLE_QUOTE {
        return Some(DOUBLE_QUOTE);
    }
    ESCAPES
        .iter()
        .find(|(escape, _)| *escape == marker)
        .map(|(_, target)| *target)
}
