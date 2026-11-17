const OPEN_MARK: &str = "{{";
const CLOSE_MARK: &str = "}}";
pub(super) const MAX_EXPANDED_BYTES: usize = 1024 * 1024;

use super::VariableScope;

pub(super) fn sorted_unique(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names.dedup();
    names
}

pub fn placeholder_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(found) = Placeholder::find(rest) {
        names.push(found.name.to_string());
        rest = found.after;
    }
    sorted_unique(names)
}

pub(super) struct Expansion<'a> {
    pub(super) scope: &'a VariableScope,
    pub(super) budget: usize,
    pub(super) active: Vec<String>,
    pub(super) unresolved: Vec<String>,
    pub(super) circular: Vec<String>,
    pub(super) limit_exceeded: bool,
}

impl Expansion<'_> {
    pub(super) fn expand(&mut self, text: &str) -> String {
        let mut output = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(found) = Placeholder::find(rest) {
            output.push_str(found.before);
            output.push_str(&self.expand_placeholder(&found));
            rest = found.after;
        }
        output.push_str(rest);
        output
    }

    fn expand_placeholder(&mut self, found: &Placeholder<'_>) -> String {
        if self.active.iter().any(|name| name == found.name) {
            self.circular.push(found.name.to_string());
            return found.literal.to_string();
        }
        let Some(value) = self.scope.lookup(found.name) else {
            self.unresolved.push(found.name.to_string());
            return found.literal.to_string();
        };
        if value.len() > self.budget {
            self.limit_exceeded = true;
            return found.literal.to_string();
        }
        self.budget -= value.len();
        self.active.push(found.name.to_string());
        let expanded = self.expand(value);
        self.active.pop();
        expanded
    }
}

pub(super) struct Placeholder<'a> {
    pub(super) before: &'a str,
    pub(super) literal: &'a str,
    pub(super) name: &'a str,
    pub(super) after: &'a str,
}

impl<'a> Placeholder<'a> {
    pub(super) fn find(text: &'a str) -> Option<Self> {
        let mut from = 0;
        while let Some(relative) = text[from..].find(CLOSE_MARK) {
            let close = from + relative;
            if let Some(found) = Self::ending_at(text, from, close) {
                return Some(found);
            }
            from = close + CLOSE_MARK.len();
        }
        None
    }

    fn ending_at(text: &'a str, from: usize, close: usize) -> Option<Self> {
        let open = from + text[from..close].rfind(OPEN_MARK)?;
        let name = text[open + OPEN_MARK.len()..close].trim();
        if name.is_empty() {
            return None;
        }
        let end = close + CLOSE_MARK.len();
        Some(Placeholder {
            before: &text[..open],
            literal: &text[open..end],
            name,
            after: &text[end..],
        })
    }
}
