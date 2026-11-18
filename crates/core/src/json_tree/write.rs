use serde::ser::{Serialize, SerializeMap, Serializer};

use super::JsonNode;

impl Serialize for JsonNode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            JsonNode::Null => serializer.serialize_unit(),
            JsonNode::Bool(value) => serializer.serialize_bool(*value),
            JsonNode::Number(number) => number.serialize(serializer),
            JsonNode::String(text) => serializer.serialize_str(text),
            JsonNode::Array(items) => items.serialize(serializer),
            JsonNode::Object(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

impl JsonNode {
    pub fn to_text(&self, pretty: bool) -> String {
        written(self, pretty)
    }

    pub fn list_text(nodes: &[&JsonNode], pretty: bool) -> String {
        written(&nodes, pretty)
    }
}

fn written<T: Serialize>(value: &T, pretty: bool) -> String {
    let text = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };
    text.unwrap_or_default()
}
