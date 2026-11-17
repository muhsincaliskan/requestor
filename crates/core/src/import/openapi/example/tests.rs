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
    let root = json!({"m": {"examples": {"a": {"value": {"x": 1}}}, "schema": {"type": "string"}}});
    let mut resolver = Resolver::new(&root);
    assert_eq!(
        media_sample(&mut resolver, &root["m"]),
        Some(json!({"x": 1}))
    );
}
