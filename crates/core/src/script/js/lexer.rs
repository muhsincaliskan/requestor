use serde_json::Number;

const PUNCTUATION: [&str; 26] = [
    "===", "!==", "=>", "==", "!=", ">=", "<=", "&&", "||", "?.", "(", ")", "{", "}", "[", "]",
    ".", ",", ";", "!", ">", "<", "=", "+", "-", "/",
];
const LINE_COMMENT: &str = "//";
const BLOCK_COMMENT_OPEN: &str = "/*";
const STAR: &str = "*";

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Kind {
    Name(String),
    Text(String),
    Number(Number),
    Punct(&'static str),
    Comment(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Token {
    pub(super) kind: Kind,
    pub(super) line: usize,
}

pub(super) fn tokens(source: &str) -> Result<Vec<Token>, (usize, String)> {
    let mut lexer = Lexer {
        found: Vec::new(),
        line: 1,
    };
    let mut rest = source;
    while let Some(next) = rest.chars().next() {
        rest = lexer
            .step(rest, next)
            .map_err(|message| (lexer.line, message))?;
    }
    Ok(lexer.found)
}

struct Lexer {
    found: Vec<Token>,
    line: usize,
}

impl Lexer {
    fn step<'a>(&mut self, rest: &'a str, next: char) -> Result<&'a str, String> {
        if next == '\n' {
            self.line += 1;
            return Ok(&rest[1..]);
        }
        if next.is_whitespace() {
            return Ok(&rest[next.len_utf8()..]);
        }
        if let Some(comment) = rest.strip_prefix(LINE_COMMENT) {
            let end = comment.find('\n').unwrap_or(comment.len());
            self.push(Kind::Comment(comment[..end].trim().to_string()));
            return Ok(&comment[end..]);
        }
        if rest.starts_with(BLOCK_COMMENT_OPEN) {
            return Err("block comments are not supported".to_string());
        }
        if matches!(next, '"' | '\'' | '`') {
            return self.text(rest, next);
        }
        if next.is_ascii_digit() {
            return self.number(rest);
        }
        if next.is_alphabetic() || next == '_' || next == '$' {
            return Ok(self.name(rest));
        }
        self.punct(rest)
    }

    fn push(&mut self, kind: Kind) {
        self.found.push(Token {
            kind,
            line: self.line,
        });
    }

    fn text<'a>(&mut self, rest: &'a str, quote: char) -> Result<&'a str, String> {
        let mut value = String::new();
        let mut characters = rest[1..].char_indices();
        while let Some((index, character)) = characters.next() {
            match character {
                _ if character == quote => {
                    self.push(Kind::Text(value));
                    return Ok(&rest[1 + index + character.len_utf8()..]);
                }
                '\\' => value.push(unescape(characters.next().map(|(_, escaped)| escaped))?),
                '$' if quote == '`' && rest[1 + index..].starts_with("${") => {
                    return Err("template expressions are not supported".to_string())
                }
                '\n' => return Err("text is missing its closing quote".to_string()),
                other => value.push(other),
            }
        }
        Err("text is missing its closing quote".to_string())
    }

    fn number<'a>(&mut self, rest: &'a str) -> Result<&'a str, String> {
        let end = rest
            .find(|character: char| !(character.is_ascii_digit() || character == '.'))
            .unwrap_or(rest.len());
        let number =
            serde_json::from_str(&rest[..end]).map_err(|_| "unreadable number".to_string())?;
        self.push(Kind::Number(number));
        Ok(&rest[end..])
    }

    fn name<'a>(&mut self, rest: &'a str) -> &'a str {
        let end = rest
            .find(|character: char| {
                !(character.is_alphanumeric() || character == '_' || character == '$')
            })
            .unwrap_or(rest.len());
        self.push(Kind::Name(rest[..end].to_string()));
        &rest[end..]
    }

    fn punct<'a>(&mut self, rest: &'a str) -> Result<&'a str, String> {
        if let Some(after) = rest.strip_prefix(STAR) {
            self.push(Kind::Punct(STAR));
            return Ok(after);
        }
        let found = PUNCTUATION
            .iter()
            .find(|mark| rest.starts_with(**mark))
            .ok_or_else(|| format!("unexpected \"{}\"", rest.chars().next().unwrap_or_default()))?;
        self.push(Kind::Punct(found));
        Ok(&rest[found.len()..])
    }
}

fn unescape(escaped: Option<char>) -> Result<char, String> {
    match escaped {
        Some('n') => Ok('\n'),
        Some('t') => Ok('\t'),
        Some('r') => Ok('\r'),
        Some(other @ ('"' | '\'' | '`' | '\\' | '/')) => Ok(other),
        _ => Err("unsupported escape in text".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{tokens, Kind};

    fn kinds(source: &str) -> Vec<Kind> {
        tokens(source)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    #[test]
    fn javascript_splits_into_names_texts_and_marks() {
        assert_eq!(
            kinds("pm.test('ok', () => x === 2.5);"),
            vec![
                Kind::Name("pm".into()),
                Kind::Punct("."),
                Kind::Name("test".into()),
                Kind::Punct("("),
                Kind::Text("ok".into()),
                Kind::Punct(","),
                Kind::Punct("("),
                Kind::Punct(")"),
                Kind::Punct("=>"),
                Kind::Name("x".into()),
                Kind::Punct("==="),
                Kind::Number(serde_json::from_str("2.5").unwrap()),
                Kind::Punct(")"),
                Kind::Punct(";"),
            ]
        );
    }

    #[test]
    fn comments_and_lines_are_tracked() {
        let found = tokens("// hello\n\nx").unwrap();
        assert_eq!(found[0].kind, Kind::Comment("hello".into()));
        assert_eq!(found[1].line, 3);
        assert!(tokens("`a ${b}`").is_err());
        assert!(tokens("/* x */").is_err());
    }
}
