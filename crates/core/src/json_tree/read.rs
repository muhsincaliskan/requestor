use std::fmt;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Number;

use super::JsonNode;

const EXPECTED: &str = "a JSON value";

struct NodeVisitor;

impl<'de> Deserialize<'de> for JsonNode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(NodeVisitor)
    }
}

impl<'de> Visitor<'de> for NodeVisitor {
    type Value = JsonNode;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(EXPECTED)
    }

    fn visit_bool<E>(self, value: bool) -> Result<JsonNode, E> {
        Ok(JsonNode::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<JsonNode, E> {
        Ok(JsonNode::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<JsonNode, E> {
        Ok(JsonNode::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<JsonNode, E> {
        Ok(Number::from_f64(value).map_or(JsonNode::Null, JsonNode::Number))
    }

    fn visit_str<E>(self, value: &str) -> Result<JsonNode, E> {
        Ok(JsonNode::String(value.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<JsonNode, E> {
        Ok(JsonNode::String(value))
    }

    fn visit_unit<E>(self) -> Result<JsonNode, E> {
        Ok(JsonNode::Null)
    }

    fn visit_none<E>(self) -> Result<JsonNode, E> {
        Ok(JsonNode::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<JsonNode, D::Error> {
        JsonNode::deserialize(deserializer)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<JsonNode, A::Error> {
        let mut items = Vec::with_capacity(access.size_hint().unwrap_or_default());
        while let Some(item) = access.next_element()? {
            items.push(item);
        }
        Ok(JsonNode::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<JsonNode, A::Error> {
        let mut entries = Vec::with_capacity(access.size_hint().unwrap_or_default());
        while let Some(entry) = access.next_entry()? {
            entries.push(entry);
        }
        Ok(JsonNode::Object(entries))
    }
}
