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

    pub(super) fn positions(&self, node: &JsonNode, found: &mut Vec<usize>) {
        match self {
            Selector::Name(name) => found.extend(node.member_position(name)),
            Selector::Index(index) => found.extend(node.element_position(*index)),
            Selector::Slice(slice) => slice.positions(node, found),
            Selector::Wildcard => found.extend(0..node.child_count()),
            Selector::Filter(filter) => found.extend(
                node.children()
                    .enumerate()
                    .filter(|(_, child)| filter.matches(child))
                    .map(|(position, _)| position),
            ),
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

    fn positions(&self, node: &JsonNode, found: &mut Vec<usize>) {
        let JsonNode::Array(items) = node else {
            return;
        };
        let length = i64::try_from(items.len()).unwrap_or(i64::MAX);
        let Ok(stride) = usize::try_from(self.step.unsigned_abs()) else {
            return;
        };
        if self.step > 0 {
            let start = forward_bound(self.start.unwrap_or(0), length);
            let end = forward_bound(self.end.unwrap_or(length), length);
            found.extend((start..end).step_by(stride).map(as_position));
        } else if self.step < 0 {
            let start = self
                .start
                .map_or(length - 1, |value| backward_bound(value, length));
            let end = self.end.map_or(-1, |value| backward_bound(value, length));
            found.extend(((end + 1)..=start).rev().step_by(stride).map(as_position));
        }
    }
}

fn forward_bound(value: i64, length: i64) -> i64 {
    if value < 0 {
        length.saturating_add(value).max(0)
    } else {
        value.min(length)
    }
}

fn backward_bound(value: i64, length: i64) -> i64 {
    if value < 0 {
        length.saturating_add(value).max(-1)
    } else {
        value.min(length - 1)
    }
}

fn as_position(index: i64) -> usize {
    usize::try_from(index).unwrap_or_default()
}
