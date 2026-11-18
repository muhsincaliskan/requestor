mod read;
mod write;

use serde_json::Number;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonNode {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<JsonNode>),
    Object(Vec<(String, JsonNode)>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonKey<'a> {
    Name(&'a str),
    Index(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonKind {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

impl JsonNode {
    pub fn parse(text: &str) -> Option<JsonNode> {
        serde_json::from_str(text).ok()
    }

    pub fn kind(&self) -> JsonKind {
        match self {
            JsonNode::Null => JsonKind::Null,
            JsonNode::Bool(_) => JsonKind::Bool,
            JsonNode::Number(_) => JsonKind::Number,
            JsonNode::String(_) => JsonKind::String,
            JsonNode::Array(_) => JsonKind::Array,
            JsonNode::Object(_) => JsonKind::Object,
        }
    }

    pub fn is_container(&self) -> bool {
        matches!(self, JsonNode::Array(_) | JsonNode::Object(_))
    }

    pub fn child_count(&self) -> usize {
        match self {
            JsonNode::Array(items) => items.len(),
            JsonNode::Object(entries) => entries.len(),
            _ => 0,
        }
    }

    pub fn child(&self, position: usize) -> Option<(JsonKey<'_>, &JsonNode)> {
        match self {
            JsonNode::Array(items) => items
                .get(position)
                .map(|item| (JsonKey::Index(position), item)),
            JsonNode::Object(entries) => entries
                .get(position)
                .map(|(name, value)| (JsonKey::Name(name), value)),
            _ => None,
        }
    }

    pub fn children(&self) -> impl Iterator<Item = &JsonNode> {
        (0..self.child_count()).filter_map(|position| self.child(position).map(|(_, node)| node))
    }

    pub fn member(&self, name: &str) -> Option<&JsonNode> {
        match self {
            JsonNode::Object(entries) => entries
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn element(&self, index: i64) -> Option<&JsonNode> {
        let JsonNode::Array(items) = self else {
            return None;
        };
        let position = if index < 0 {
            items.len().checked_sub(index.unsigned_abs() as usize)?
        } else {
            usize::try_from(index).ok()?
        };
        items.get(position)
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            JsonNode::Number(number) => number.as_f64(),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            JsonNode::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn summary(&self) -> String {
        match self {
            JsonNode::Array(items) => format!("[{}]", items.len()),
            JsonNode::Object(entries) => format!("{{{}}}", entries.len()),
            scalar => scalar.to_text(false),
        }
    }
}
