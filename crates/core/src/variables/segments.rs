use super::expansion::Placeholder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSegment<'a> {
    Plain(&'a str),
    Placeholder { literal: &'a str, name: &'a str },
}

impl TextSegment<'_> {
    pub fn text(&self) -> &str {
        match self {
            TextSegment::Plain(text) => text,
            TextSegment::Placeholder { literal, .. } => literal,
        }
    }

    pub fn is_known_in(&self, names: &[String]) -> bool {
        match self {
            TextSegment::Plain(_) => false,
            TextSegment::Placeholder { name, .. } => names.iter().any(|known| known == name),
        }
    }
}

pub struct TextSegments<'a> {
    rest: &'a str,
    pending: Option<TextSegment<'a>>,
}

impl<'a> TextSegments<'a> {
    pub fn new(text: &'a str) -> Self {
        TextSegments {
            rest: text,
            pending: None,
        }
    }

    pub fn placeholder_at(text: &'a str, char_index: usize) -> Option<&'a str> {
        let mut start = 0;
        for segment in TextSegments::new(text) {
            let end = start + segment.text().chars().count();
            if let TextSegment::Placeholder { name, .. } = segment {
                if (start..=end).contains(&char_index) {
                    return Some(name);
                }
            }
            start = end;
        }
        None
    }
}

impl<'a> Iterator for TextSegments<'a> {
    type Item = TextSegment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pending) = self.pending.take() {
            return Some(pending);
        }
        if self.rest.is_empty() {
            return None;
        }
        let Some(found) = Placeholder::find(self.rest) else {
            let plain = TextSegment::Plain(self.rest);
            self.rest = "";
            return Some(plain);
        };
        self.rest = found.after;
        let placeholder = TextSegment::Placeholder {
            literal: found.literal,
            name: found.name,
        };
        if found.before.is_empty() {
            return Some(placeholder);
        }
        self.pending = Some(placeholder);
        Some(TextSegment::Plain(found.before))
    }
}
