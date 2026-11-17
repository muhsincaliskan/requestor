use super::ResponseBuffer;

#[derive(Default)]
pub struct BodySearch {
    pub query: String,
    searched: Option<String>,
    matches: Vec<usize>,
    current: usize,
    pending_scroll: Option<usize>,
}

impl BodySearch {
    pub fn refresh(&mut self, buffer: &ResponseBuffer) {
        if self.searched.as_deref() == Some(self.query.as_str()) {
            return;
        }
        self.matches = buffer.rows_containing(&self.query);
        self.current = 0;
        self.searched = Some(self.query.clone());
        self.pending_scroll = self.current_row();
    }

    pub fn invalidate(&mut self) {
        self.searched = None;
    }

    pub fn step(&mut self, forward: bool) {
        let count = self.matches.len();
        if count == 0 {
            return;
        }
        self.current = if forward {
            (self.current + 1) % count
        } else {
            (self.current + count - 1) % count
        };
        self.pending_scroll = self.current_row();
    }

    pub fn current_row(&self) -> Option<usize> {
        self.matches.get(self.current).copied()
    }

    pub fn is_match(&self, row: usize) -> bool {
        self.matches.binary_search(&row).is_ok()
    }

    pub fn take_scroll_target(&mut self) -> Option<usize> {
        self.pending_scroll.take()
    }

    pub fn position(&self) -> Option<(usize, usize)> {
        (!self.query.is_empty()).then_some((
            (self.current + 1).min(self.matches.len()),
            self.matches.len(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::BodySearch;
    use crate::response::ResponseBuffer;

    #[test]
    fn steps_through_matches_and_wraps_around() {
        let buffer = ResponseBuffer::from_text("a\nb\na\nb\na".to_string());
        let mut search = BodySearch {
            query: "a".to_string(),
            ..BodySearch::default()
        };
        search.refresh(&buffer);
        assert_eq!(search.position(), Some((1, 3)));
        assert_eq!(search.take_scroll_target(), Some(0));
        search.step(false);
        assert_eq!(search.current_row(), Some(4));
        search.step(true);
        assert_eq!(search.current_row(), Some(0));
        assert!(search.is_match(2) && !search.is_match(1));
    }

    #[test]
    fn an_empty_query_has_no_position_and_a_new_query_searches_again() {
        let buffer = ResponseBuffer::from_text("x\ny".to_string());
        let mut search = BodySearch::default();
        search.refresh(&buffer);
        assert_eq!(search.position(), None);
        search.query = "y".to_string();
        search.refresh(&buffer);
        assert_eq!(search.current_row(), Some(1));
    }
}
