use super::parse::condition::{COMPARISONS, SUBJECT_WORDS};
use super::Phase;

const COMMANDS: [&str; 9] = [
    "set", "clear", "expect", "save", "fail", "if", "else if", "else", "end",
];
const CONDITION_STARTERS: [&str; 6] = ["expect", "if", "and", "or", "not", "save"];
const TARGETS: [&str; 3] = ["environment", "collection", "request"];
const SET_VALUES: [&str; 5] = [
    "new uuid",
    "timestamp",
    "timestamp ms",
    "random number 1 100",
    "now iso",
];
const AFTER_CONDITION: [&str; 3] = ["and", "or", "named"];
const WORD_GAP: char = ' ';
const SUBJECTS_WITH_NAME: [&str; 2] = ["header", "json"];

pub struct Completion<'a> {
    pub phase: Phase,
    pub variables: &'a [String],
}

impl Completion<'_> {
    pub fn suggestions(&self, line_before_cursor: &str) -> Vec<String> {
        let (done, partial) = split_partial(line_before_cursor);
        let words: Vec<&str> = done.split_whitespace().collect();
        self.candidates(&words)
            .into_iter()
            .filter(|candidate| candidate.starts_with(partial) && candidate != partial)
            .collect()
    }

    pub fn insertion(line_before_cursor: &str, candidate: &str) -> String {
        let (_, partial) = split_partial(line_before_cursor);
        let rest = candidate.strip_prefix(partial).unwrap_or(candidate);
        format!("{rest}{WORD_GAP}")
    }

    fn candidates(&self, words: &[&str]) -> Vec<String> {
        let Some(last) = words.last() else {
            return owned(&COMMANDS);
        };
        let previous = words.len().checked_sub(2).map(|index| words[index]);
        match (previous, *last) {
            (_, "in") => owned(&TARGETS),
            (_, "to") => owned(&SET_VALUES),
            (_, "set" | "clear" | "as") => self.variable_names(),
            (_, word) if CONDITION_STARTERS.contains(&word) => self.subjects(),
            (Some(word), _) if SUBJECTS_WITH_NAME.contains(&word) => self.after_subject(words),
            (_, word) if is_subject(word) && !SUBJECTS_WITH_NAME.contains(&word) => {
                self.after_subject(words)
            }
            (_, word) if ends_comparison(word) => Vec::new(),
            _ if words.first() != Some(&"set") => owned(&AFTER_CONDITION),
            _ => vec!["in".to_string()],
        }
    }

    fn subjects(&self) -> Vec<String> {
        let mut found = match self.phase {
            Phase::AfterResponse => owned(&SUBJECT_WORDS),
            Phase::BeforeRequest => Vec::new(),
        };
        found.extend(self.variables.iter().map(|name| format!("{{{{{name}}}}}")));
        found
    }

    fn after_subject(&self, words: &[&str]) -> Vec<String> {
        if words.first() == Some(&"save") {
            return vec!["as".to_string()];
        }
        COMPARISONS
            .iter()
            .map(|(words, _)| words.join(" "))
            .collect()
    }

    fn variable_names(&self) -> Vec<String> {
        self.variables.to_vec()
    }
}

fn split_partial(line_before_cursor: &str) -> (&str, &str) {
    let trimmed = line_before_cursor.trim_start();
    match trimmed.rfind(char::is_whitespace) {
        Some(split) => (&trimmed[..split], &trimmed[split + 1..]),
        None => ("", trimmed),
    }
}

fn is_subject(word: &str) -> bool {
    SUBJECT_WORDS.contains(&word) || word.starts_with("{{")
}

fn ends_comparison(word: &str) -> bool {
    COMPARISONS
        .iter()
        .any(|(words, _)| words.last() == Some(&word))
}

fn owned(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::Completion;
    use crate::script::Phase;

    fn suggest(line: &str, phase: Phase) -> Vec<String> {
        let variables = vec!["token".to_string()];
        Completion {
            phase,
            variables: &variables,
        }
        .suggestions(line)
    }

    #[test]
    fn suggestions_follow_the_sentence() {
        assert_eq!(suggest("ex", Phase::AfterResponse), vec!["expect"]);
        assert!(suggest("expect ", Phase::AfterResponse).contains(&"status".to_string()));
        assert_eq!(suggest("expect ", Phase::BeforeRequest), vec!["{{token}}"]);
        assert!(suggest("expect status ", Phase::AfterResponse).contains(&"is not".to_string()));
        assert!(
            suggest("expect header X-Id ", Phase::AfterResponse).contains(&"exists".to_string())
        );
        assert_eq!(
            suggest("save json data.id ", Phase::AfterResponse),
            vec!["as"]
        );
        assert_eq!(
            suggest("save json data.id as id in c", Phase::AfterResponse),
            vec!["collection"]
        );
        assert_eq!(suggest("set ", Phase::BeforeRequest), vec!["token"]);
        assert!(suggest("set a to ", Phase::BeforeRequest).contains(&"new uuid".to_string()));
    }

    #[test]
    fn insertion_finishes_the_partial_word() {
        assert_eq!(Completion::insertion("  ex", "expect"), "pect ");
        assert_eq!(Completion::insertion("expect ", "status"), "status ");
    }
}
