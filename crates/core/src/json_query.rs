mod cursor;
mod filter;
mod selector;

use std::fmt;

use cursor::Cursor;
use selector::Selector;

use crate::json_tree::walk::extended;
use crate::json_tree::{JsonNode, NodePath};

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
        let leading = text.chars().count() - text.trim_start().chars().count();
        JsonQuery::parse_trimmed(text.trim()).map_err(|error| InvalidQuery {
            position: error.position + leading,
        })
    }

    fn parse_trimmed(text: &str) -> Result<JsonQuery, InvalidQuery> {
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
        self.select_paths(root)
            .iter()
            .filter_map(|path| root.at_path(path))
            .collect()
    }

    pub fn select_paths(&self, root: &JsonNode) -> Vec<NodePath> {
        let start = vec![(NodePath::new(), root)];
        self.segments
            .iter()
            .fold(start, |nodes, segment| segment.apply(&nodes))
            .into_iter()
            .map(|(path, _)| path)
            .collect()
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

    fn apply<'a>(&self, nodes: &[(NodePath, &'a JsonNode)]) -> Vec<(NodePath, &'a JsonNode)> {
        let mut found = Vec::new();
        for (path, node) in nodes {
            if self.descendants {
                for (relative, visited) in node.descendants() {
                    let mut full = path.clone();
                    full.extend(relative);
                    self.select_from(&full, visited, &mut found);
                }
            } else {
                self.select_from(path, node, &mut found);
            }
        }
        found
    }

    fn select_from<'a>(
        &self,
        path: &[usize],
        node: &'a JsonNode,
        found: &mut Vec<(NodePath, &'a JsonNode)>,
    ) {
        let mut positions = Vec::new();
        for selector in &self.selectors {
            selector.positions(node, &mut positions);
        }
        found.extend(positions.into_iter().filter_map(|position| {
            let (_, child) = node.child(position)?;
            Some((extended(path, position), child))
        }));
    }
}
