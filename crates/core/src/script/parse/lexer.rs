use serde_json::Number;

use super::super::literal::split_quoted;
use crate::variables::OPEN_MARK;

const QUOTE: char = '"';
const CLOSE_MARK: &str = "}}";

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Token {
    Word(String),
    Text(String),
    Number(Number),
    Variable(String),
}

pub(super) fn tokens(line: &str) -> Result<Vec<Token>, String> {
    let mut found = Vec::new();
    let mut rest = line.trim_start();
    while !rest.is_empty() {
        let (token, remaining) = next_token(rest)?;
        found.push(token);
        rest = remaining.trim_start();
    }
    Ok(found)
}

fn next_token(text: &str) -> Result<(Token, &str), String> {
    if text.starts_with(QUOTE) {
        let (value, rest) = split_quoted(text).ok_or("text is missing its closing quote")?;
        return Ok((Token::Text(value), rest));
    }
    if let Some(inner) = text.strip_prefix(OPEN_MARK) {
        let close = inner
            .find(CLOSE_MARK)
            .ok_or("{{ is missing its closing }}")?;
        let name = inner[..close].trim();
        if name.is_empty() {
            return Err("{{}} needs a variable name".to_string());
        }
        return Ok((
            Token::Variable(name.to_string()),
            &inner[close + CLOSE_MARK.len()..],
        ));
    }
    let end = text
        .find(|character: char| character.is_whitespace() || character == QUOTE)
        .unwrap_or(text.len());
    let word = &text[..end];
    let token = match serde_json::from_str::<Number>(word) {
        Ok(number) => Token::Number(number),
        Err(_) => Token::Word(word.to_string()),
    };
    Ok((token, &text[end..]))
}

#[cfg(test)]
mod tests {
    use serde_json::Number;

    use super::{tokens, Token};

    fn word(text: &str) -> Token {
        Token::Word(text.to_string())
    }

    #[test]
    fn lines_split_into_words_texts_numbers_and_variables() {
        assert_eq!(
            tokens(r#"set auth to "Bearer {{t}}" in collection"#).unwrap(),
            vec![
                word("set"),
                word("auth"),
                word("to"),
                Token::Text("Bearer {{t}}".to_string()),
                word("in"),
                word("collection"),
            ]
        );
        assert_eq!(
            tokens("if {{env}} is -2.5").unwrap(),
            vec![
                word("if"),
                Token::Variable("env".to_string()),
                word("is"),
                Token::Number(Number::from_f64(-2.5).unwrap()),
            ]
        );
        assert_eq!(
            tokens("expect json data.items[0].id exists").unwrap()[2],
            word("data.items[0].id")
        );
    }

    #[test]
    fn unfinished_texts_and_variables_are_errors() {
        assert!(tokens(r#"fail "oops"#).is_err());
        assert!(tokens("if {{env is 1").is_err());
        assert!(tokens("if {{ }} is 1").is_err());
    }
}
