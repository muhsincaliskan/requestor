const INDENT: &str = "  ";
const NEWLINE: char = '\n';
const MAX_INDENT_LEVELS: usize = 32;
const PLACEHOLDER_OPEN: &str = "{{";
const PLACEHOLDER_CLOSE: &str = "}}";

struct Layout {
    output: String,
    depth: usize,
    in_string: bool,
    in_placeholder: bool,
    escaped: bool,
    pending_open: Option<char>,
}

pub fn reindent(text: &str) -> String {
    let mut layout = Layout {
        output: String::with_capacity(text.len() + text.len() / 4),
        depth: 0,
        in_string: false,
        in_placeholder: false,
        escaped: false,
        pending_open: None,
    };
    for character in text.chars() {
        layout.accept(character);
    }
    layout.flush_pending_open();
    layout.output
}

impl Layout {
    fn accept(&mut self, character: char) {
        if self.in_string {
            self.accept_in_string(character);
        } else if self.in_placeholder {
            self.accept_in_placeholder(character);
        } else if !character.is_whitespace() {
            self.accept_structural(character);
        }
    }

    fn accept_in_string(&mut self, character: char) {
        self.output.push(character);
        if self.escaped {
            self.escaped = false;
        } else if character == '\\' {
            self.escaped = true;
        } else if character == '"' {
            self.in_string = false;
        }
    }

    fn accept_in_placeholder(&mut self, character: char) {
        self.output.push(character);
        if self.output.ends_with(PLACEHOLDER_CLOSE) {
            self.in_placeholder = false;
        }
    }

    fn accept_structural(&mut self, character: char) {
        match character {
            '{' if self.pending_open == Some('{') => self.open_placeholder(),
            '}' | ']' => self.close(character),
            _ => {
                self.flush_pending_open();
                self.accept_token(character);
            }
        }
    }

    fn accept_token(&mut self, character: char) {
        match character {
            '{' | '[' => self.pending_open = Some(character),
            ',' => {
                self.output.push(character);
                self.break_line();
            }
            ':' => self.output.push_str(": "),
            '"' => {
                self.in_string = true;
                self.output.push(character);
            }
            other => self.output.push(other),
        }
    }

    fn open_placeholder(&mut self) {
        self.pending_open = None;
        self.output.push_str(PLACEHOLDER_OPEN);
        self.in_placeholder = true;
    }

    fn close(&mut self, character: char) {
        if let Some(open) = self.pending_open.take() {
            self.output.push(open);
        } else {
            self.depth = self.depth.saturating_sub(1);
            self.break_line();
        }
        self.output.push(character);
    }

    fn flush_pending_open(&mut self) {
        if let Some(open) = self.pending_open.take() {
            self.output.push(open);
            self.depth += 1;
            self.break_line();
        }
    }

    fn break_line(&mut self) {
        self.output.push(NEWLINE);
        for _ in 0..self.depth.min(MAX_INDENT_LEVELS) {
            self.output.push_str(INDENT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::reindent;

    #[test]
    fn deep_nesting_does_not_multiply_the_size() {
        let depth = 4_000;
        let document = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let laid_out = reindent(&document);
        let longest_line = 2 * super::MAX_INDENT_LEVELS + 2;
        assert!(laid_out.len() <= 2 * depth * longest_line);
        assert_eq!(laid_out.lines().count(), 2 * depth - 1);
    }

    #[test]
    fn minified_documents_are_spread_over_lines() {
        assert_eq!(
            reindent(r#"{"a":1,"b":[true,null,{"c":"d"}],"e":{}}"#),
            "{\n  \"a\": 1,\n  \"b\": [\n    true,\n    null,\n    {\n      \"c\": \"d\"\n    }\n  ],\n  \"e\": {}\n}"
        );
    }

    #[test]
    fn string_contents_are_left_alone() {
        let text = r#"{"k":"a,b:{c}[d] \" \\","n":" spaced "}"#;
        let layout = reindent(text);
        assert!(layout.contains(r#""a,b:{c}[d] \" \\""#));
        assert!(layout.contains(r#"" spaced ""#));
    }

    #[test]
    fn the_result_is_the_same_document_and_stable() {
        let text = r#" [ {"a" : [ ] , "b":{"c":[1,2,{"d":"é }"}]}} , "x" ] "#;
        let first = reindent(text);
        let original: serde_json::Value = serde_json::from_str(text).unwrap();
        let laid_out: serde_json::Value = serde_json::from_str(&first).unwrap();
        assert_eq!(original, laid_out);
        assert_eq!(reindent(&first), first);
    }

    #[test]
    fn variable_placeholders_stay_intact() {
        assert_eq!(
            reindent(r#"{"id":{{petId}},"tags":[{{ firstTag }},"b"]}"#),
            "{\n  \"id\": {{petId}},\n  \"tags\": [\n    {{ firstTag }},\n    \"b\"\n  ]\n}"
        );
        assert_eq!(reindent("{{body}}"), "{{body}}");
    }

    #[test]
    fn damaged_documents_do_not_panic() {
        for text in ["}}]]", "{\"open", "[1,,", "", "\"\\", "{{", "{{x"] {
            let _ = reindent(text);
        }
    }
}
