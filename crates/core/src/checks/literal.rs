const QUOTE: char = '"';
const ESCAPE: char = '\\';

pub fn quote(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_default()
}

pub fn split_quoted(text: &str) -> Option<(String, &str)> {
    let body = text.strip_prefix(QUOTE)?;
    let mut escaped = false;
    for (index, character) in body.char_indices() {
        match character {
            _ if escaped => escaped = false,
            ESCAPE => escaped = true,
            QUOTE => {
                let end = QUOTE.len_utf8() + index + QUOTE.len_utf8();
                let value = serde_json::from_str(&text[..end]).ok()?;
                return Some((value, &text[end..]));
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{quote, split_quoted};

    #[test]
    fn quoted_strings_round_trip_with_escapes() {
        let quoted = quote("say \"hi\"\n");
        let text = format!("{quoted}, rest");
        assert_eq!(
            split_quoted(&text),
            Some(("say \"hi\"\n".to_string(), ", rest"))
        );
        assert_eq!(split_quoted("\"open"), None);
        assert_eq!(split_quoted("bare"), None);
    }
}
