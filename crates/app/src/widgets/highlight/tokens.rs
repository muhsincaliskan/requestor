use std::ops::Range;

use requestor_core::response::ResponseFormat;

const JSON_PUNCTUATION: &str = "{}[],:";
const NUMBER_CHARS: &str = "0123456789.eE+-";
const JSON_KEYWORDS: [&str; 3] = ["true", "false", "null"];
const COMMENT_OPEN: &str = "<!--";
const COMMENT_CLOSE: &str = "-->";
const TAG_PREFIXES: [char; 3] = ['/', '?', '!'];
const NAME_PUNCTUATION: &str = "-_:.";
const ATTRIBUTE_STOPS: &str = "=>/\"'<";
const QUOTES: [char; 2] = ['"', '\''];
const SELF_CLOSING_PREFIXES: [char; 2] = ['/', '?'];
const TAG_CLOSE: char = '>';
const JSON_ESCAPE: char = '\\';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Key,
    String,
    Number,
    Keyword,
    Punctuation,
    Tag,
    Attribute,
    AttributeValue,
    Comment,
    Text,
}

#[derive(Clone, Copy)]
enum Grammar {
    Json,
    Markup,
    Plain,
}

impl From<ResponseFormat> for Grammar {
    fn from(format: ResponseFormat) -> Self {
        match format {
            ResponseFormat::Json => Grammar::Json,
            ResponseFormat::Xml | ResponseFormat::Html => Grammar::Markup,
            ResponseFormat::Text | ResponseFormat::Binary => Grammar::Plain,
        }
    }
}

pub struct Tokens<'a> {
    row: &'a str,
    position: usize,
    grammar: Grammar,
    inside_tag: bool,
}

impl<'a> Tokens<'a> {
    pub fn new(row: &'a str, format: ResponseFormat) -> Self {
        Tokens {
            row,
            position: 0,
            grammar: Grammar::from(format),
            inside_tag: false,
        }
    }
}

impl Iterator for Tokens<'_> {
    type Item = (Range<usize>, TokenKind);

    fn next(&mut self) -> Option<Self::Item> {
        let rest = self
            .row
            .get(self.position..)
            .filter(|rest| !rest.is_empty())?;
        let (length, kind) = match self.grammar {
            Grammar::Json => json_token(rest),
            Grammar::Markup => markup_token(rest, &mut self.inside_tag),
            Grammar::Plain => (rest.len(), TokenKind::Text),
        };
        let range = self.position..self.position + length;
        self.position = range.end;
        Some((range, kind))
    }
}

fn first_char(text: &str) -> char {
    text.chars().next().unwrap_or_default()
}

fn run(text: &str, belongs: impl Fn(char) -> bool) -> usize {
    let length = text
        .find(|character| !belongs(character))
        .unwrap_or(text.len());
    length.max(first_char(text).len_utf8())
}

fn quoted_len(text: &str, escapes: bool) -> usize {
    let quote = first_char(text);
    let mut characters = text.char_indices().skip(1);
    while let Some((index, character)) = characters.next() {
        if escapes && character == JSON_ESCAPE {
            characters.next();
        } else if character == quote {
            return index + 1;
        }
    }
    text.len()
}

fn json_token(rest: &str) -> (usize, TokenKind) {
    let first = first_char(rest);
    match first {
        '"' => json_string(rest),
        '-' | '0'..='9' => (run(rest, |c| NUMBER_CHARS.contains(c)), TokenKind::Number),
        _ if first.is_alphabetic() => json_word(rest),
        _ if JSON_PUNCTUATION.contains(first) => (1, TokenKind::Punctuation),
        _ if first.is_whitespace() => (run(rest, char::is_whitespace), TokenKind::Text),
        _ => (run(rest, |c| !starts_json_token(c)), TokenKind::Text),
    }
}

fn starts_json_token(character: char) -> bool {
    character == '"'
        || character == '-'
        || character.is_ascii_digit()
        || character.is_alphabetic()
        || character.is_whitespace()
        || JSON_PUNCTUATION.contains(character)
}

fn json_string(rest: &str) -> (usize, TokenKind) {
    let length = quoted_len(rest, true);
    let followed_by_colon = rest[length..]
        .trim_start_matches(char::is_whitespace)
        .starts_with(':');
    let kind = if followed_by_colon {
        TokenKind::Key
    } else {
        TokenKind::String
    };
    (length, kind)
}

fn json_word(rest: &str) -> (usize, TokenKind) {
    let length = run(rest, char::is_alphanumeric);
    let kind = if JSON_KEYWORDS.contains(&&rest[..length]) {
        TokenKind::Keyword
    } else {
        TokenKind::Text
    };
    (length, kind)
}

fn markup_token(rest: &str, inside_tag: &mut bool) -> (usize, TokenKind) {
    if rest.starts_with(COMMENT_OPEN) {
        let length = rest
            .find(COMMENT_CLOSE)
            .map_or(rest.len(), |at| at + COMMENT_CLOSE.len());
        return (length, TokenKind::Comment);
    }
    if rest.starts_with('<') {
        *inside_tag = true;
        return (tag_open_len(rest), TokenKind::Tag);
    }
    if *inside_tag {
        return tag_body_token(rest, inside_tag);
    }
    (run(rest, |c| c != '<'), TokenKind::Text)
}

fn tag_open_len(rest: &str) -> usize {
    let after = &rest[1..];
    let prefix = usize::from(after.starts_with(TAG_PREFIXES));
    let name = after[prefix..]
        .find(|c: char| !(c.is_alphanumeric() || NAME_PUNCTUATION.contains(c)))
        .unwrap_or(after.len() - prefix);
    1 + prefix + name
}

fn tag_body_token(rest: &str, inside_tag: &mut bool) -> (usize, TokenKind) {
    let first = first_char(rest);
    if first == TAG_CLOSE {
        *inside_tag = false;
        return (1, TokenKind::Tag);
    }
    if SELF_CLOSING_PREFIXES.contains(&first) && rest[1..].starts_with(TAG_CLOSE) {
        *inside_tag = false;
        return (2, TokenKind::Tag);
    }
    match first {
        _ if first.is_whitespace() => (run(rest, char::is_whitespace), TokenKind::Text),
        _ if QUOTES.contains(&first) => (quoted_len(rest, false), TokenKind::AttributeValue),
        '=' | '/' => (1, TokenKind::Punctuation),
        _ => (run(rest, is_attribute_char), TokenKind::Attribute),
    }
}

fn is_attribute_char(character: char) -> bool {
    !character.is_whitespace() && !ATTRIBUTE_STOPS.contains(character)
}
