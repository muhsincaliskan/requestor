pub const BYTES_PER_MEGABYTE: usize = 1024 * 1024;
const MAX_ROW_BYTES: usize = 2_000;
const LINE_BREAK: u8 = b'\n';

pub struct ResponseBuffer {
    text: String,
    row_starts: Vec<usize>,
}

impl ResponseBuffer {
    pub fn from_text(text: String) -> Self {
        let mut row_starts = Vec::new();
        let mut line_start = 0;
        for (position, _) in text.match_indices(LINE_BREAK as char) {
            push_rows(&text, line_start, position, &mut row_starts);
            line_start = position + 1;
        }
        if line_start < text.len() {
            push_rows(&text, line_start, text.len(), &mut row_starts);
        }
        ResponseBuffer { text, row_starts }
    }

    pub fn row_count(&self) -> usize {
        self.row_starts.len()
    }

    pub fn byte_len(&self) -> usize {
        self.text.len()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn row(&self, index: usize) -> &str {
        let start = self.row_starts[index];
        let end = self
            .row_starts
            .get(index + 1)
            .map_or(self.text.len(), |next| *next);
        self.text[start..end].trim_end_matches(['\r', '\n'])
    }
}

fn push_rows(text: &str, line_start: usize, line_end: usize, row_starts: &mut Vec<usize>) {
    let mut start = line_start;
    row_starts.push(start);
    while line_end - start > MAX_ROW_BYTES {
        let mut cut = start + MAX_ROW_BYTES;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        row_starts.push(cut);
        start = cut;
    }
}

#[cfg(test)]
mod tests {
    use super::{ResponseBuffer, MAX_ROW_BYTES};

    #[test]
    fn indexes_lines_with_or_without_a_trailing_newline() {
        let buffer = ResponseBuffer::from_text("a\nbb\nccc\n".to_string());
        assert_eq!(buffer.row_count(), 3);
        assert_eq!(buffer.row(0), "a");
        assert_eq!(buffer.row(2), "ccc");
        let open = ResponseBuffer::from_text("a\nb".to_string());
        assert_eq!(open.row_count(), 2);
        assert_eq!(open.row(1), "b");
    }

    #[test]
    fn keeps_blank_lines_and_strips_carriage_returns() {
        let buffer = ResponseBuffer::from_text("a\r\n\r\nb\r\n".to_string());
        assert_eq!(buffer.row_count(), 3);
        assert_eq!(buffer.row(0), "a");
        assert_eq!(buffer.row(1), "");
        assert_eq!(buffer.row(2), "b");
    }

    #[test]
    fn empty_text_has_no_rows() {
        assert_eq!(ResponseBuffer::from_text(String::new()).row_count(), 0);
    }

    #[test]
    fn very_long_lines_are_split_on_character_boundaries() {
        let line = "é".repeat(MAX_ROW_BYTES);
        let buffer = ResponseBuffer::from_text(format!("{line}\nshort"));
        assert_eq!(buffer.row_count(), 3);
        let rejoined: String = (0..2).map(|index| buffer.row(index)).collect();
        assert_eq!(rejoined, line);
        assert!(buffer.row(0).len() <= MAX_ROW_BYTES);
        assert_eq!(buffer.row(2), "short");
    }
}
