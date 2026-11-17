use std::fmt::{Display, Write};

#[derive(Default)]
pub struct JsonLine {
    fields: Vec<String>,
}

impl JsonLine {
    pub fn new() -> Self {
        JsonLine::default()
    }

    pub fn text(mut self, key: &str, value: &str) -> Self {
        self.fields
            .push(format!("{}:{}", quoted(key), quoted(value)));
        self
    }

    pub fn texts(mut self, key: &str, values: &[String]) -> Self {
        let items: Vec<String> = values.iter().map(|value| quoted(value)).collect();
        self.fields
            .push(format!("{}:[{}]", quoted(key), items.join(",")));
        self
    }

    pub fn raw(mut self, key: &str, value: impl Display) -> Self {
        self.fields.push(format!("{}:{value}", quoted(key)));
        self
    }

    pub fn render(&self) -> String {
        format!("{{{}}}", self.fields.join(","))
    }
}

fn quoted(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() + 2);
    escaped.push('"');
    for character in text.chars() {
        push_escaped(&mut escaped, character);
    }
    escaped.push('"');
    escaped
}

fn push_escaped(target: &mut String, character: char) {
    match character {
        '"' => target.push_str("\\\""),
        '\\' => target.push_str("\\\\"),
        '\n' => target.push_str("\\n"),
        '\r' => target.push_str("\\r"),
        '\t' => target.push_str("\\t"),
        control if control.is_control() => {
            let _ = write!(target, "\\u{:04x}", u32::from(control));
        }
        other => target.push(other),
    }
}

#[cfg(test)]
mod tests {
    use super::JsonLine;

    #[test]
    fn renders_fields_in_order() {
        let line = JsonLine::new().text("a", "x").raw("b", 3).raw("c", true);
        assert_eq!(line.render(), r#"{"a":"x","b":3,"c":true}"#);
    }

    #[test]
    fn escapes_quotes_backslashes_and_controls() {
        let line = JsonLine::new().text("k", "a\"b\\c\nd\u{1}");
        assert_eq!(line.render(), r#"{"k":"a\"b\\c\nd\u0001"}"#);
    }
}
