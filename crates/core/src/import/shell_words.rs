use super::ImportError;

pub(super) fn split(command: &str) -> Result<Vec<String>, ImportError> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut characters = command.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\'' => read_single_quoted(&mut characters, &mut current, &mut started)?,
            '"' => read_double_quoted(&mut characters, &mut current, &mut started)?,
            '$' if characters.peek() == Some(&'\'') => {
                characters.next();
                read_ansi_quoted(&mut characters, &mut current, &mut started)?;
            }
            '\\' => read_unquoted_escape(&mut characters, &mut current, &mut started),
            space if space.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            other => {
                started = true;
                current.push(other);
            }
        }
    }
    if started {
        words.push(current);
    }
    Ok(words)
}

type Characters<'a> = std::iter::Peekable<std::str::Chars<'a>>;

fn skip_line_break(characters: &mut Characters) -> bool {
    let mut ahead = characters.clone();
    let skipped = match ahead.next() {
        Some('\n') => 1,
        Some('\r') if ahead.next() == Some('\n') => 2,
        _ => return false,
    };
    characters.nth(skipped - 1);
    true
}

fn read_unquoted_escape(characters: &mut Characters, current: &mut String, started: &mut bool) {
    if skip_line_break(characters) {
        return;
    }
    *started = true;
    current.extend(characters.next());
}

fn read_single_quoted(
    characters: &mut Characters,
    current: &mut String,
    started: &mut bool,
) -> Result<(), ImportError> {
    *started = true;
    for character in characters.by_ref() {
        if character == '\'' {
            return Ok(());
        }
        current.push(character);
    }
    Err(ImportError::UnterminatedQuote)
}

fn read_double_quoted(
    characters: &mut Characters,
    current: &mut String,
    started: &mut bool,
) -> Result<(), ImportError> {
    *started = true;
    while let Some(character) = characters.next() {
        match character {
            '"' => return Ok(()),
            '\\' if skip_line_break(characters) => {}
            '\\' => match characters.peek().copied() {
                Some(escaped @ ('"' | '\\' | '$' | '`')) => {
                    current.push(escaped);
                    characters.next();
                }
                _ => current.push('\\'),
            },
            other => current.push(other),
        }
    }
    Err(ImportError::UnterminatedQuote)
}

fn read_ansi_quoted(
    characters: &mut Characters,
    current: &mut String,
    started: &mut bool,
) -> Result<(), ImportError> {
    *started = true;
    while let Some(character) = characters.next() {
        match character {
            '\'' => return Ok(()),
            '\\' => current.push(ansi_escape(characters.next().unwrap_or('\\'))),
            other => current.push(other),
        }
    }
    Err(ImportError::UnterminatedQuote)
}

fn ansi_escape(escaped: char) -> char {
    match escaped {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::split;
    use crate::import::ImportError;

    #[test]
    fn splits_plain_single_double_and_ansi_quoted_words() {
        let words = split(r#"curl -H 'A: b c' "x \"y\" $z" $'l1\nl2' plain\ word"#).unwrap();
        assert_eq!(
            words,
            ["curl", "-H", "A: b c", "x \"y\" $z", "l1\nl2", "plain word"]
        );
    }

    #[test]
    fn joins_line_continuations() {
        assert_eq!(
            split("curl \\\n  -X POST \\\r\n  url").unwrap(),
            ["curl", "-X", "POST", "url"]
        );
    }

    #[test]
    fn line_continuations_respect_quoting() {
        assert_eq!(split("a 'x\\\ny' b").unwrap(), ["a", "x\\\ny", "b"]);
        assert_eq!(split("a \"x\\\ny\" b").unwrap(), ["a", "xy", "b"]);
        assert_eq!(split("ab\\\ncd").unwrap(), ["abcd"]);
    }

    #[test]
    fn keeps_empty_quoted_words() {
        assert_eq!(split("a '' b").unwrap(), ["a", "", "b"]);
    }

    #[test]
    fn unterminated_quotes_are_errors() {
        assert_eq!(
            split("curl 'oops").unwrap_err(),
            ImportError::UnterminatedQuote
        );
    }
}
