use std::ops::Range;

use super::ResponseBuffer;

#[derive(Default)]
pub struct BodySelection {
    anchor: Option<usize>,
    head: usize,
    dragging: bool,
}

impl BodySelection {
    pub fn begin(&mut self, offset: usize) {
        self.anchor = Some(offset);
        self.head = offset;
        self.dragging = true;
    }

    pub fn extend(&mut self, offset: usize) {
        if self.dragging {
            self.head = offset;
        }
    }

    pub fn finish(&mut self) {
        self.dragging = false;
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn select_all(&mut self, buffer: &ResponseBuffer) {
        self.anchor = Some(0);
        self.head = buffer.text().len();
        self.dragging = false;
    }

    pub fn clear(&mut self) {
        *self = BodySelection::default();
    }

    pub fn range(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        let range = anchor.min(self.head)..anchor.max(self.head);
        (!range.is_empty()).then_some(range)
    }

    pub fn text<'a>(&self, buffer: &'a ResponseBuffer) -> Option<&'a str> {
        self.range().and_then(|range| buffer.text().get(range))
    }

    pub fn columns_in_row(&self, buffer: &ResponseBuffer, row: usize) -> Option<Range<usize>> {
        let selected = self.range()?;
        let span = buffer.row_span(row);
        let start = selected.start.max(span.start);
        let end = selected.end.min(span.end);
        (start < end).then(|| buffer.column_at(row, start)..buffer.column_at(row, end))
    }
}

#[cfg(test)]
mod tests {
    use super::BodySelection;
    use crate::response::ResponseBuffer;

    #[test]
    fn a_drag_across_rows_selects_the_text_between_in_either_direction() {
        let buffer = ResponseBuffer::from_text("{\n  \"ağ\": 1\n}".to_string());
        let mut selection = BodySelection::default();
        selection.begin(buffer.offset_at(1, 3));
        selection.extend(buffer.offset_at(2, 1));
        selection.finish();
        assert_eq!(selection.text(&buffer), Some("ağ\": 1\n}"));
        assert_eq!(selection.columns_in_row(&buffer, 1), Some(3..9));
        assert_eq!(selection.columns_in_row(&buffer, 0), None);
        selection.begin(buffer.offset_at(1, 5));
        selection.extend(buffer.offset_at(0, 0));
        assert_eq!(selection.text(&buffer), Some("{\n  \"ağ"));
        selection.finish();
        selection.extend(buffer.offset_at(2, 1));
        assert_eq!(selection.text(&buffer), Some("{\n  \"ağ"));
    }

    #[test]
    fn select_all_covers_the_whole_body_and_clear_drops_it() {
        let buffer = ResponseBuffer::from_text("a\nb".to_string());
        let mut selection = BodySelection::default();
        selection.select_all(&buffer);
        assert_eq!(selection.text(&buffer), Some("a\nb"));
        selection.clear();
        assert_eq!(selection.text(&buffer), None);
    }
}
