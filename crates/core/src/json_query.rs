mod cursor;
mod filter;
mod selector;

use std::fmt;

use cursor::Cursor;
use selector::Selector;

use crate::json_tree::JsonNode;

const ROOT: &str = "$";
const DESCENDANTS: &str = "..";
const MEMBER: &str = ".";
const WILDCARD: &str = "*";
const BRACKET_OPEN: &str = "[";
const BRACKET_CLOSE: &str = "]";
const SELECTOR_SEPARATOR: &str = ",";

#[derive(Debug, Clone, PartialEq)]
pub struct JsonQuery {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq)]
struct Segment {
    descendants: bool,
    selectors: Vec<Selector>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidQuery {
    pub position: usize,
}

impl fmt::Display for InvalidQuery {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "Not a valid JSONPath at character {}",
            self.position
        )
    }
}

impl JsonQuery {
    pub fn parse(text: &str) -> Result<JsonQuery, InvalidQuery> {
        let text = text.trim();
        let mut cursor = Cursor::new(text);
        let explicit =
            cursor.eat(ROOT) || text.starts_with(MEMBER) || text.starts_with(BRACKET_OPEN);
        let implicit_member = !explicit && !text.is_empty();
        let mut segments = Vec::new();
        if implicit_member {
            segments.push(Segment::member(&mut cursor)?);
        }
        while !cursor.is_done() {
            segments.push(Segment::parse(&mut cursor)?);
        }
        Ok(JsonQuery { segments })
    }

    pub fn select<'a>(&self, root: &'a JsonNode) -> Vec<&'a JsonNode> {
        self.segments
            .iter()
            .fold(vec![root], |nodes, segment| segment.apply(&nodes))
    }
}

impl Segment {
    fn parse(cursor: &mut Cursor) -> Result<Segment, InvalidQuery> {
        if cursor.eat(DESCENDANTS) {
            let mut segment = if cursor.rest().starts_with(BRACKET_OPEN) {
                Segment::bracketed(cursor)?
            } else {
                Segment::member(cursor)?
            };
            segment.descendants = true;
            return Ok(segment);
        }
        if cursor.eat(MEMBER) {
            return Segment::member(cursor);
        }
        if cursor.rest().starts_with(BRACKET_OPEN) {
            return Segment::bracketed(cursor);
        }
        Err(cursor.invalid())
    }

    fn member(cursor: &mut Cursor) -> Result<Segment, InvalidQuery> {
        let selector = if cursor.eat(WILDCARD) {
            Selector::Wildcard
        } else {
            Selector::Name(cursor.name()?)
        };
        Ok(Segment::of(vec![selector]))
    }

    fn bracketed(cursor: &mut Cursor) -> Result<Segment, InvalidQuery> {
        cursor.expect(BRACKET_OPEN)?;
        let mut selectors = vec![Selector::parse_bracketed(cursor)?];
        cursor.skip_space();
        while cursor.eat(SELECTOR_SEPARATOR) {
            selectors.push(Selector::parse_bracketed(cursor)?);
            cursor.skip_space();
        }
        cursor.expect(BRACKET_CLOSE)?;
        Ok(Segment::of(selectors))
    }

    fn of(selectors: Vec<Selector>) -> Segment {
        Segment {
            descendants: false,
            selectors,
        }
    }

    fn apply<'a>(&self, nodes: &[&'a JsonNode]) -> Vec<&'a JsonNode> {
        let mut found = Vec::new();
        for node in nodes {
            if self.descendants {
                for visited in self_and_descendants(node) {
                    self.select_from(visited, &mut found);
                }
            } else {
                self.select_from(node, &mut found);
            }
        }
        found
    }

    fn select_from<'a>(&self, node: &'a JsonNode, found: &mut Vec<&'a JsonNode>) {
        for selector in &self.selectors {
            selector.apply(node, found);
        }
    }
}

fn self_and_descendants(node: &JsonNode) -> Vec<&JsonNode> {
    let mut visited = Vec::new();
    let mut pending = vec![node];
    while let Some(current) = pending.pop() {
        visited.push(current);
        let children: Vec<&JsonNode> = current.children().collect();
        pending.extend(children.into_iter().rev());
    }
    visited
}
