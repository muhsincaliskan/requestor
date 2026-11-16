pub struct ResponseBuffer {
    text: String,
    line_starts: Vec<u32>,
}

impl ResponseBuffer {
    pub fn from_text(text: String) -> Self {
        let line_starts = std::iter::once(0)
            .chain(
                text.match_indices('\n')
                    .map(|(position, _)| position as u32 + 1),
            )
            .filter(|start| (*start as usize) < text.len())
            .collect();
        ResponseBuffer { text, line_starts }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn byte_len(&self) -> usize {
        self.text.len()
    }

    pub fn line(&self, index: usize) -> &str {
        let start = self.line_starts[index] as usize;
        let end = self
            .line_starts
            .get(index + 1)
            .map_or(self.text.len(), |next| *next as usize);
        self.text[start..end].trim_end_matches('\n')
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
    fn empty_text_has_no_lines() {
        assert_eq!(ResponseBuffer::from_text(String::new()).line_count(), 0);
    }
}
