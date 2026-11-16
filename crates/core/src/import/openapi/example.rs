use std::collections::HashMap;

use serde_json::{Map, Value};

use super::reference::Resolver;

const MAX_SAMPLE_DEPTH: usize = 8;
const MAX_SAMPLE_NODES: usize = 10_000;
const SCHEMA_KEY: &str = "schema";
const STRING_FALLBACK: &str = "string";
const NULL_TYPE: &str = "null";
const FORMAT_PLACEHOLDERS: [(&str, &str); 6] = [
    ("date-time", "2024-01-01T00:00:00Z"),
    ("date", "2024-01-01"),
    ("email", "user@example.com"),
    ("uuid", "00000000-0000-0000-0000-000000000000"),
    ("uri", "https://example.com"),
    ("binary", ""),
];
const ALTERNATIVE_KEYWORDS: [&str; 2] = ["oneOf", "anyOf"];
const DECLARED_VALUE_KEYWORDS: [&str; 3] = ["example", "default", "const"];

pub fn explicit_example<'a>(resolver: &mut Resolver<'a>, holder: &'a Value) -> Option<Value> {
    if let Some(example) = holder.get("example") {
        return Some(example.clone());
    }
    let examples = holder.get("examples")?.as_object()?;
    let first = examples.iter().min_by_key(|(name, _)| name.as_str())?.1;
    resolver.resolve(first)?.get("value").cloned()
}

pub fn declared_value(schema: &Value) -> Option<Value> {
    DECLARED_VALUE_KEYWORDS
        .iter()
        .find_map(|key| schema.get(key))
        .or_else(|| schema.get("enum")?.as_array()?.first())
        .or_else(|| schema.get("examples")?.as_array()?.first())
        .cloned()
}

pub fn media_sample<'a>(resolver: &mut Resolver<'a>, media: &'a Value) -> Option<Value> {
    explicit_example(resolver, media).or_else(|| {
        let schema = media.get(SCHEMA_KEY)?;
        Some(schema_sample(resolver, schema))
    })
}

pub fn schema_sample<'a>(resolver: &mut Resolver<'a>, schema: &'a Value) -> Value {
    Sampler {
        resolver,
        active: Vec::new(),
        cache: HashMap::new(),
        remaining: MAX_SAMPLE_NODES,
        truncated: false,
    }
    .value(schema, 0)
}

struct Sampler<'r, 'a> {
    resolver: &'r mut Resolver<'a>,
    active: Vec<&'a str>,
    cache: HashMap<&'a str, Value>,
    remaining: usize,
    truncated: bool,
}

impl<'a> Sampler<'_, 'a> {
    fn value(&mut self, schema: &'a Value, depth: usize) -> Value {
        if self.remaining == 0 {
            return self.truncate();
        }
        self.remaining -= 1;
        if depth > MAX_SAMPLE_DEPTH {
            return self.truncate();
        }
        match Resolver::reference_of(schema) {
            Some(reference) => self.referenced(schema, reference, depth),
            None => self.resolved(schema, depth),
        }
    }

    fn truncate(&mut self) -> Value {
        self.truncated = true;
        Value::Null
    }

    fn referenced(&mut self, schema: &'a Value, reference: &'a str, depth: usize) -> Value {
        if self.active.contains(&reference) {
            return self.truncate();
        }
        if let Some(cached) = self.cache.get(reference).cloned() {
            return self.reuse(cached);
        }
        self.active.push(reference);
        let outer_truncated = std::mem::replace(&mut self.truncated, false);
        let sample = self.resolved(schema, depth);
        self.active.pop();
        if !self.truncated {
            self.cache.insert(reference, sample.clone());
        }
        self.truncated |= outer_truncated;
        sample
    }

    fn reuse(&mut self, cached: Value) -> Value {
        let size = node_count(&cached);
        if size > self.remaining {
            return self.truncate();
        }
        self.remaining -= size;
        cached
    }

    fn resolved(&mut self, schema: &'a Value, depth: usize) -> Value {
        let Some(schema) = self.resolver.resolve(schema) else {
            return Value::Null;
        };
        declared_value(schema)
            .or_else(|| self.composed(schema, depth))
            .unwrap_or_else(|| self.typed(schema, depth))
    }

    fn composed(&mut self, schema: &'a Value, depth: usize) -> Option<Value> {
        if let Some(parts) = schema.get("allOf").and_then(Value::as_array) {
            return Some(self.merged(parts, depth));
        }
        let first = ALTERNATIVE_KEYWORDS
            .iter()
            .find_map(|keyword| schema.get(keyword)?.as_array()?.first())?;
        Some(self.value(first, depth + 1))
    }

    fn merged(&mut self, parts: &'a [Value], depth: usize) -> Value {
        let mut merged = Map::new();
        for part in parts {
            if let Value::Object(fields) = self.value(part, depth + 1) {
                merged.extend(fields);
            }
        }
        Value::Object(merged)
    }

    fn typed(&mut self, schema: &'a Value, depth: usize) -> Value {
        match schema_type(schema) {
            Some("object") => self.object(schema, depth),
            Some("array") => self.array(schema, depth),
            Some("string") => Value::String(string_placeholder(schema)),
            Some("integer" | "number") => Value::from(0),
            Some("boolean") => Value::Bool(false),
            _ if schema.get("properties").is_some() => self.object(schema, depth),
            _ => Value::Null,
        }
    }

    fn object(&mut self, schema: &'a Value, depth: usize) -> Value {
        let properties = schema.get("properties").and_then(Value::as_object);
        let mut fields = Map::new();
        for (name, property) in properties.into_iter().flatten() {
            if self.remaining == 0 {
                self.truncated = true;
                break;
            }
            fields.insert(name.clone(), self.value(property, depth + 1));
        }
        Value::Object(fields)
    }

    fn array(&mut self, schema: &'a Value, depth: usize) -> Value {
        let items = schema
            .get("items")
            .map(|items| self.value(items, depth + 1));
        Value::Array(items.into_iter().collect())
    }
}

fn node_count(value: &Value) -> usize {
    let children: usize = match value {
        Value::Array(items) => items.iter().map(node_count).sum(),
        Value::Object(fields) => fields.values().map(node_count).sum(),
        _ => 0,
    };
    1 + children
}

fn schema_type(schema: &Value) -> Option<&str> {
    match schema.get("type")? {
        Value::String(name) => Some(name),
        Value::Array(names) => names
            .iter()
            .filter_map(Value::as_str)
            .find(|name| *name != NULL_TYPE),
        _ => None,
    }
}

fn string_placeholder(schema: &Value) -> String {
    let format = schema.get("format").and_then(Value::as_str);
    FORMAT_PLACEHOLDERS
        .iter()
        .find(|(name, _)| Some(*name) == format)
        .map_or(STRING_FALLBACK, |(_, placeholder)| placeholder)
        .to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::{media_sample, node_count, schema_sample, MAX_SAMPLE_NODES};
    use crate::import::openapi::reference::Resolver;

    fn sample(root: &Value, pointer: &str) -> Value {
        let mut resolver = Resolver::new(root);
        schema_sample(&mut resolver, root.pointer(pointer).unwrap())
    }

    #[test]
    fn builds_a_skeleton_from_types_formats_and_enums() {
        let root = json!({"s": {"type": "object", "properties": {
            "id": {"type": "integer"},
            "status": {"type": "string", "enum": ["sold", "new"]},
            "born": {"type": "string", "format": "date"},
            "tags": {"type": "array", "items": {"type": "string"}},
            "limit": {"type": "integer", "default": 20}
        }}});
        assert_eq!(
            sample(&root, "/s"),
            json!({"id": 0, "status": "sold", "born": "2024-01-01", "tags": ["string"], "limit": 20})
        );
    }

    #[test]
    fn self_referencing_schemas_terminate() {
        let root = json!({"node": {"type": "object", "properties": {
            "next": {"$ref": "#/node"}, "name": {"type": "string"}}}});
        assert_eq!(
            sample(&root, "/node"),
            json!({"next": {"name": "string", "next": null}, "name": "string"})
        );
    }

    #[test]
    fn wide_deep_reference_chains_stay_within_the_node_budget() {
        let mut schemas = serde_json::Map::new();
        for level in 0..9 {
            let properties: serde_json::Map<String, Value> = (0..10)
                .map(|index| {
                    let target = format!("#/s{}", level + 1);
                    (format!("p{index}"), json!({ "$ref": target }))
                })
                .collect();
            schemas.insert(
                format!("s{level}"),
                json!({"type": "object", "properties": properties}),
            );
        }
        schemas.insert("s9".to_string(), json!({"type": "string"}));
        let root = Value::Object(schemas);
        let started = std::time::Instant::now();
        let result = sample(&root, "/s0");
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        assert!(node_count(&result) <= MAX_SAMPLE_NODES);
    }

    #[test]
    fn shared_references_are_sampled_once_and_reused() {
        let root = json!({"s": {"type": "object", "properties": {
            "a": {"$ref": "#/leaf"}, "b": {"$ref": "#/leaf"}}},
            "leaf": {"type": "object", "properties": {"n": {"type": "integer"}}}});
        assert_eq!(sample(&root, "/s"), json!({"a": {"n": 0}, "b": {"n": 0}}));
    }

    #[test]
    fn explicit_examples_win_over_schemas() {
        let root =
            json!({"m": {"examples": {"a": {"value": {"x": 1}}}, "schema": {"type": "string"}}});
        let mut resolver = Resolver::new(&root);
        assert_eq!(
            media_sample(&mut resolver, &root["m"]),
            Some(json!({"x": 1}))
        );
    }
}
