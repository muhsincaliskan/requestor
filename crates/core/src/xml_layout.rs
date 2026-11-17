const INDENT: &str = "  ";
const MAX_INDENT_LEVELS: usize = 32;
const TAG_OPEN: char = '<';
const TAG_CLOSE: char = '>';
const CLOSING_PREFIX: &str = "</";
const SELF_CLOSING_SUFFIX: &str = "/>";
const STANDALONE_PREFIXES: [&str; 2] = ["<?", "<!"];
const DELIMITED_MARKUP: [(&str, &str); 2] = [("<!--", "-->"), ("<![CDATA[", "]]>")];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token<'a> {
    Open(&'a str),
    Close(&'a str),
    Standalone(&'a str),
    Text(&'a str),
}

pub fn reindent(text: &str) -> String {
    let Some(tokens) = tokenize(text) else {
        return text.to_string();
    };
    let mut layout = Layout {
        output: String::with_capacity(text.len() + text.len() / 4),
        depth: 0,
    };
    let mut rest = tokens.as_slice();
    while !rest.is_empty() {
        let consumed = layout.place(rest);
        rest = &rest[consumed..];
    }
    layout.output
}

impl<'a> Token<'a> {
    fn markup(text: &'a str) -> Self {
        if text.starts_with(CLOSING_PREFIX) {
            Token::Close(text)
        } else if text.ends_with(SELF_CLOSING_SUFFIX)
            || STANDALONE_PREFIXES
                .iter()
                .any(|prefix| text.starts_with(prefix))
        {
            Token::Standalone(text)
        } else {
            Token::Open(text)
        }
    }

    fn text(self) -> &'a str {
        match self {
            Token::Open(text)
            | Token::Close(text)
            | Token::Standalone(text)
            | Token::Text(text) => text,
        }
    }
}

struct Layout {
    output: String,
    depth: usize,
}

impl Layout {
    fn place(&mut self, tokens: &[Token]) -> usize {
        match tokens {
            [open @ Token::Open(_), text @ Token::Text(_), close @ Token::Close(_), ..] => {
                self.line(&[*open, *text, *close]);
                3
            }
            [open @ Token::Open(_), close @ Token::Close(_), ..] => {
                self.line(&[*open, *close]);
                2
            }
            [open @ Token::Open(_), ..] => {
                self.line(&[*open]);
                self.depth += 1;
                1
            }
            [close @ Token::Close(_), ..] => {
                self.depth = self.depth.saturating_sub(1);
                self.line(&[*close]);
                1
            }
            [single, ..] => {
                self.line(&[*single]);
                1
            }
            [] => 0,
        }
    }

    fn line(&mut self, tokens: &[Token]) {
        if !self.output.is_empty() {
            self.output.push('\n');
        }
        self.output
            .push_str(&INDENT.repeat(self.depth.min(MAX_INDENT_LEVELS)));
        for token in tokens {
            self.output.push_str(token.text());
        }
    }
}

fn tokenize(text: &str) -> Option<Vec<Token<'_>>> {
    let mut tokens = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let markup_start = rest.find(TAG_OPEN).unwrap_or(rest.len());
        let content = rest[..markup_start].trim();
        if !content.is_empty() {
            tokens.push(Token::Text(content));
        }
        rest = &rest[markup_start..];
        if rest.is_empty() {
            break;
        }
        let markup_end = markup_end(rest)?;
        tokens.push(Token::markup(&rest[..markup_end]));
        rest = &rest[markup_end..];
    }
    Some(tokens)
}

fn markup_end(markup: &str) -> Option<usize> {
    for (opener, closer) in DELIMITED_MARKUP {
        if let Some(body) = markup.strip_prefix(opener) {
            return body
                .find(closer)
                .map(|end| opener.len() + end + closer.len());
        }
    }
    tag_end(markup)
}

fn tag_end(markup: &str) -> Option<usize> {
    let mut quote = None;
    for (index, character) in markup.char_indices() {
        match (quote, character) {
            (None, '"' | '\'') => quote = Some(character),
            (Some(open), _) if open == character => quote = None,
            (None, TAG_CLOSE) => return Some(index + TAG_CLOSE.len_utf8()),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::reindent;

    #[test]
    fn nests_elements_and_keeps_short_text_inline() {
        let laid_out =
            reindent(r#"<?xml version="1.0"?><a x="1"><b>hi {{name}}</b><c/><d></d></a>"#);
        assert_eq!(
            laid_out,
            "<?xml version=\"1.0\"?>\n<a x=\"1\">\n  <b>hi {{name}}</b>\n  <c/>\n  <d></d>\n</a>"
        );
        assert_eq!(reindent(&laid_out), laid_out);
    }

    #[test]
    fn comments_cdata_and_quoted_brackets_stay_whole() {
        let laid_out = reindent(r#"<a t="x>y"><!-- <b> --><![CDATA[<raw>]]></a>"#);
        assert_eq!(
            laid_out,
            "<a t=\"x>y\">\n  <!-- <b> -->\n  <![CDATA[<raw>]]>\n</a>"
        );
    }

    #[test]
    fn unterminated_markup_is_left_unchanged() {
        assert_eq!(reindent("<a><b"), "<a><b");
        assert_eq!(reindent("<a><!-- open"), "<a><!-- open");
    }
}
