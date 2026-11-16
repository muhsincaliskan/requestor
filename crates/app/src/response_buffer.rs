pub const BYTES_PER_MEGABYTE: usize = 1024 * 1024;

pub struct ResponseBuffer {
    text: String,
    line_starts: Vec<usize>,
}

impl ResponseBuffer {
    pub fn from_text(text: String) -> Self {
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(position, _)| position + 1))
            .filter(|start| *start < text.len())
            .collect();
        ResponseBuffer { text, line_starts }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn megabytes(&self) -> f64 {
        self.text.len() as f64 / BYTES_PER_MEGABYTE as f64
    }

    pub fn line(&self, index: usize) -> &str {
        let start = self.line_starts[index];
        let end = self
            .line_starts
            .get(index + 1)
            .map_or(self.text.len(), |next| *next);
        self.text[start..end].trim_end_matches(['\r', '\n'])
    }
}

#[cfg(test)]
mod tests {
    use super::ResponseBuffer;

    #[test]
    fn indexes_lines_without_trailing_newline() {
        let buffer = ResponseBuffer::from_text("a\nbb\nccc\n".to_string());
        assert_eq!(buffer.line_count(), 3);
        assert_eq!(buffer.line(0), "a");
        assert_eq!(buffer.line(2), "ccc");
    }

    #[test]
    fn keeps_last_line_without_terminator() {
        let buffer = ResponseBuffer::from_text("a\nb".to_string());
        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.line(1), "b");
    }

    #[test]
    fn strips_carriage_returns() {
        let buffer = ResponseBuffer::from_text("a\r\nb\r\n".to_string());
        assert_eq!(buffer.line(0), "a");
        assert_eq!(buffer.line(1), "b");
    }

    #[test]
    fn empty_text_has_no_lines() {
        assert_eq!(ResponseBuffer::from_text(String::new()).line_count(), 0);
    }
}
