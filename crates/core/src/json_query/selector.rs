use super::cursor::Cursor;
use super::filter::Filter;
use super::InvalidQuery;
use crate::json_tree::JsonNode;

const WILDCARD: &str = "*";
const FILTER: &str = "?";
const RANGE: &str = ":";

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Selector {
    Name(String),
    Index(i64),
    Slice(Slice),
    Wildcard,
    Filter(Filter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Slice {
    start: Option<i64>,
    end: Option<i64>,
    step: i64,
}

impl Selector {
    pub(super) fn parse_bracketed(cursor: &mut Cursor) -> Result<Selector, InvalidQuery> {
        cursor.skip_space();
        if cursor.eat(WILDCARD) {
            return Ok(Selector::Wildcard);
        }
        if cursor.eat(FILTER) {
            return Filter::parse(cursor).map(Selector::Filter);
        }
        if cursor.starts_quoted() {
            return cursor.quoted().map(Selector::Name);
        }
        let start = cursor.integer()?;
        cursor.skip_space();
        if cursor.eat(RANGE) {
            return Slice::parse(cursor, start).map(Selector::Slice);
        }
        start.map(Selector::Index).ok_or_else(|| cursor.invalid())
    }

    pub(super) fn apply<'a>(&self, node: &'a JsonNode, found: &mut Vec<&'a JsonNode>) {
        match self {
            Selector::Name(name) => found.extend(node.member(name)),
            Selector::Index(index) => found.extend(node.element(*index)),
            Selector::Slice(slice) => slice.apply(node, found),
            Selector::Wildcard => found.extend(node.children()),
            Selector::Filter(filter) => {
                found.extend(node.children().filter(|child| filter.matches(child)))
            }
        }
    }
}

impl Slice {
    fn parse(cursor: &mut Cursor, start: Option<i64>) -> Result<Slice, InvalidQuery> {
        let end = cursor.integer()?;
        cursor.skip_space();
        let step = if cursor.eat(RANGE) {
            cursor.integer()?.unwrap_or(1)
        } else {
            1
        };
        Ok(Slice { start, end, step })
    }

    fn apply<'a>(&self, node: &'a JsonNode, found: &mut Vec<&'a JsonNode>) {
        let JsonNode::Array(items) = node else {
            return;
        };
        let length = items.len() as i64;
        let bound = |value: i64| {
            if value < 0 {
                (length + value).max(0)
            } else {
                value.min(length)
            }
        };
        let indices: Vec<i64> = match self.step {
            0 => Vec::new(),
            step if step > 0 => {
                let start = bound(self.start.unwrap_or(0));
                let end = bound(self.end.unwrap_or(length));
                (start..end).step_by(step as usize).collect()
            }
            step => {
                let start = self
                    .start
                    .map_or(length - 1, |value| bound(value).min(length - 1));
                let end = self.end.map_or(-1, bound);
                let mut downward: Vec<i64> = ((end + 1)..=start).rev().collect();
                downward.retain(|index| (start - index) % step.abs() == 0);
                downward
            }
        };
        found.extend(
            indices
                .into_iter()
                .filter_map(|index| items.get(index as usize)),
        );
    }
}
