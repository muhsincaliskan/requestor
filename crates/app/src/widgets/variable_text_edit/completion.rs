const OPEN_BRACES: &str = "{{";
const CLOSE_BRACES: &str = "}}";
const LINE_BREAK: char = '\n';

pub const MAX_SUGGESTIONS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialVariable {
    start: usize,
    prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completed {
    pub text: String,
    pub cursor: usize,
}

impl PartialVariable {
    pub fn at(text: &str, cursor: usize) -> Option<Self> {
        let before: String = text.chars().take(cursor).collect();
        let open = before.rfind(OPEN_BRACES)?;
        let prefix = &before[open + OPEN_BRACES.len()..];
        if prefix.contains(CLOSE_BRACES) || prefix.contains(LINE_BREAK) {
            return None;
        }
        Some(PartialVariable {
            start: before[..open].chars().count(),
            prefix: prefix.to_string(),
        })
    }

    pub fn start(&self) -> usize {
        self.start
    }

    #[cfg(test)]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn candidates<'a>(&self, names: &'a [String], limit: usize) -> Vec<&'a str> {
        let needle = self.prefix.to_lowercase();
        let lowered: Vec<String> = names.iter().map(|name| name.to_lowercase()).collect();
        let starting = names
            .iter()
            .zip(&lowered)
            .filter(|(_, low)| low.starts_with(&needle));
        let containing = names
            .iter()
            .zip(&lowered)
            .filter(|(_, low)| !low.starts_with(&needle) && low.contains(&needle));
        starting
            .chain(containing)
            .map(|(name, _)| name.as_str())
            .take(limit)
            .collect()
    }

    pub fn complete(&self, text: &str, cursor: usize, name: &str) -> Completed {
        let chars: Vec<char> = text.chars().collect();
        let cursor = cursor.min(chars.len());
        let head: String = chars[..self.start.min(cursor)].iter().collect();
        let tail: String = chars[cursor..].iter().collect();
        let tail = tail.strip_prefix(CLOSE_BRACES).unwrap_or(&tail);
        let insertion = format!("{OPEN_BRACES}{name}{CLOSE_BRACES}");
        let cursor = head.chars().count() + insertion.chars().count();
        Completed {
            text: format!("{head}{insertion}{tail}"),
            cursor,
        }
    }
}

pub fn step_selection(current: usize, forward: bool, len: usize) -> usize {
    match (len, forward) {
        (0, _) => 0,
        (_, true) => (current + 1) % len,
        (_, false) => (current + len - 1) % len,
    }
}

#[cfg(test)]
mod tests {
    use super::{step_selection, PartialVariable};

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn finds_the_unclosed_open_braces_before_the_cursor() {
        let partial = PartialVariable::at("https://{{ho", 12).unwrap();
        assert_eq!(partial.prefix(), "ho");
        assert_eq!(partial.start(), 8);
    }

    #[test]
    fn an_empty_prefix_right_after_the_braces_is_a_partial() {
        assert_eq!(PartialVariable::at("{{", 2).unwrap().prefix(), "");
    }

    #[test]
    fn closed_or_absent_braces_are_not_partial() {
        assert!(PartialVariable::at("{{host}}/x", 10).is_none());
        assert!(PartialVariable::at("plain", 5).is_none());
        assert!(PartialVariable::at("{{a}} and {b", 12).is_none());
        assert!(PartialVariable::at("{{a\nb", 5).is_none());
    }

    #[test]
    fn only_text_before_the_cursor_counts() {
        let partial = PartialVariable::at("{{host}}", 4).unwrap();
        assert_eq!(partial.prefix(), "ho");
        assert!(PartialVariable::at("{{host", 1).is_none());
    }

    #[test]
    fn indices_are_chars_not_bytes() {
        let partial = PartialVariable::at("é€{{na", 6).unwrap();
        assert_eq!(partial.start(), 2);
        assert_eq!(partial.prefix(), "na");
    }

    #[test]
    fn candidates_prefer_prefix_matches_then_substrings_case_insensitively() {
        let all = names(&["base_url", "token", "url", "Other"]);
        let partial = PartialVariable::at("{{UR", 4).unwrap();
        assert_eq!(partial.candidates(&all, 8), ["url", "base_url"]);
        assert_eq!(partial.candidates(&all, 1), ["url"]);
    }

    #[test]
    fn an_empty_prefix_lists_everything_up_to_the_limit() {
        let all = names(&["a", "b", "c"]);
        let partial = PartialVariable::at("{{", 2).unwrap();
        assert_eq!(partial.candidates(&all, 2), ["a", "b"]);
    }

    #[test]
    fn completing_replaces_the_partial_and_moves_the_cursor_after_it() {
        let partial = PartialVariable::at("go {{ho/x", 7).unwrap();
        let done = partial.complete("go {{ho/x", 7, "host");
        assert_eq!(done.text, "go {{host}}/x");
        assert_eq!(done.cursor, 11);
    }

    #[test]
    fn completing_inside_existing_braces_does_not_duplicate_them() {
        let partial = PartialVariable::at("{{ho}}/x", 4).unwrap();
        let done = partial.complete("{{ho}}/x", 4, "host");
        assert_eq!(done.text, "{{host}}/x");
        assert_eq!(done.cursor, 8);
    }

    #[test]
    fn completing_counts_unicode_chars() {
        let partial = PartialVariable::at("é€{{na", 6).unwrap();
        let done = partial.complete("é€{{na", 6, "naïve");
        assert_eq!(done.text, "é€{{naïve}}");
        assert_eq!(done.cursor, 11);
    }

    #[test]
    fn selection_wraps_in_both_directions() {
        assert_eq!(step_selection(0, false, 3), 2);
        assert_eq!(step_selection(2, true, 3), 0);
        assert_eq!(step_selection(1, true, 3), 2);
        assert_eq!(step_selection(0, true, 0), 0);
    }
}
