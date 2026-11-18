use std::cmp::Ordering;

use super::cursor::Cursor;
use super::InvalidQuery;
use crate::json_tree::JsonNode;

const OPEN: &str = "(";
const CLOSE: &str = ")";
const CURRENT: &str = "@";
const MEMBER: &str = ".";
const BRACKET_OPEN: &str = "[";
const BRACKET_CLOSE: &str = "]";
const OPERATORS: [(&str, Comparison); 6] = [
    ("==", Comparison::Equal),
    ("!=", Comparison::NotEqual),
    ("<=", Comparison::AtMost),
    (">=", Comparison::AtLeast),
    ("<", Comparison::Less),
    (">", Comparison::Greater),
];

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Filter {
    path: Vec<Step>,
    test: Option<(Comparison, JsonNode)>,
}

#[derive(Debug, Clone, PartialEq)]
enum Step {
    Member(String),
    Element(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Comparison {
    Equal,
    NotEqual,
    Less,
    AtMost,
    Greater,
    AtLeast,
}

impl Filter {
    pub(super) fn parse(cursor: &mut Cursor) -> Result<Filter, InvalidQuery> {
        cursor.skip_space();
        let wrapped = cursor.eat(OPEN);
        cursor.expect(CURRENT)?;
        let path = steps(cursor)?;
        cursor.skip_space();
        let test = match OPERATORS.iter().find(|(symbol, _)| cursor.eat(symbol)) {
            Some((_, comparison)) => Some((*comparison, cursor.literal()?)),
            None => None,
        };
        if wrapped {
            cursor.expect(CLOSE)?;
        }
        Ok(Filter { path, test })
    }

    pub(super) fn matches(&self, node: &JsonNode) -> bool {
        let target = self.path.iter().try_fold(node, |current, step| match step {
            Step::Member(name) => current.member(name),
            Step::Element(index) => current.element(*index),
        });
        match (&self.test, target) {
            (None, found) => found.is_some(),
            (Some((Comparison::NotEqual, _)), None) => true,
            (Some(_), None) => false,
            (Some((comparison, literal)), Some(found)) => comparison.holds(found, literal),
        }
    }
}

impl Comparison {
    fn holds(self, found: &JsonNode, literal: &JsonNode) -> bool {
        let ordering = found.compare(literal);
        match self {
            Comparison::Equal => ordering == Some(Ordering::Equal),
            Comparison::NotEqual => ordering != Some(Ordering::Equal),
            Comparison::Less => ordering == Some(Ordering::Less),
            Comparison::AtMost => matches!(ordering, Some(Ordering::Less | Ordering::Equal)),
            Comparison::Greater => ordering == Some(Ordering::Greater),
            Comparison::AtLeast => matches!(ordering, Some(Ordering::Greater | Ordering::Equal)),
        }
    }
}

fn steps(cursor: &mut Cursor) -> Result<Vec<Step>, InvalidQuery> {
    let mut path = Vec::new();
    loop {
        if cursor.eat(MEMBER) {
            path.push(Step::Member(cursor.name()?));
        } else if cursor.eat(BRACKET_OPEN) {
            path.push(bracketed_step(cursor)?);
            cursor.expect(BRACKET_CLOSE)?;
        } else {
            return Ok(path);
        }
    }
}

fn bracketed_step(cursor: &mut Cursor) -> Result<Step, InvalidQuery> {
    cursor.skip_space();
    if cursor.starts_quoted() {
        return cursor.quoted().map(Step::Member);
    }
    cursor
        .integer()?
        .map(Step::Element)
        .ok_or_else(|| cursor.invalid())
}
