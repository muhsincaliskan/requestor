use serde_json::Value;

use crate::encoding::percent_decode;

pub const MAX_REFERENCE_DEPTH: usize = 32;
const REFERENCE_KEY: &str = "$ref";
const LOCAL_REFERENCE_PREFIX: char = '#';
const ENCODED_MARK: char = '%';

pub struct Resolver<'a> {
    root: &'a Value,
    warnings: Vec<String>,
}

impl<'a> Resolver<'a> {
    pub fn new(root: &'a Value) -> Self {
        Resolver {
            root,
            warnings: Vec::new(),
        }
    }

    pub fn warn(&mut self, message: impl Into<String>) {
        let message = message.into();
        if !self.warnings.contains(&message) {
            self.warnings.push(message);
        }
    }

    pub fn into_warnings(self) -> Vec<String> {
        self.warnings
    }

    pub fn reference_of(value: &Value) -> Option<&str> {
        value.get(REFERENCE_KEY)?.as_str()
    }

    pub fn resolve(&mut self, value: &'a Value) -> Option<&'a Value> {
        let mut current = value;
        let mut visited: Vec<&'a str> = Vec::new();
        while let Some(reference) = Resolver::reference_of(current) {
            if visited.len() >= MAX_REFERENCE_DEPTH || visited.contains(&reference) {
                self.warn(format!("circular reference {reference} was skipped"));
                return None;
            }
            visited.push(reference);
            current = self.lookup(reference)?;
        }
        Some(current)
    }

    fn lookup(&mut self, reference: &str) -> Option<&'a Value> {
        let Some(pointer) = reference.strip_prefix(LOCAL_REFERENCE_PREFIX) else {
            self.warn(format!("external reference {reference} was not loaded"));
            return None;
        };
        let pointer = if pointer.contains(ENCODED_MARK) {
            percent_decode(pointer)
        } else {
            pointer.to_string()
        };
        let found = self.root.pointer(&pointer);
        if found.is_none() {
            self.warn(format!("reference {reference} could not be resolved"));
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Resolver, MAX_REFERENCE_DEPTH};

    #[test]
    fn follows_local_pointers_with_escaped_segments() {
        let root = json!({"a/b": {"c~d": 1}, "ref": {"$ref": "#/a~1b/c~0d"}});
        let mut resolver = Resolver::new(&root);
        assert_eq!(resolver.resolve(&root["ref"]), Some(&json!(1)));
        assert!(resolver.into_warnings().is_empty());
    }

    #[test]
    fn cycles_and_external_references_warn_without_loading() {
        let root = json!({
            "a": {"$ref": "#/b"},
            "b": {"$ref": "#/a"},
            "external": {"$ref": "https://example.test/x.json#/y"},
            "missing": {"$ref": "#/nope"}
        });
        let mut resolver = Resolver::new(&root);
        for key in ["a", "external", "missing"] {
            assert!(resolver.resolve(&root[key]).is_none());
        }
        assert_eq!(resolver.into_warnings().len(), 3);
    }

    #[test]
    fn long_reference_chains_stop_at_the_depth_cap() {
        let mut root = serde_json::Map::new();
        for index in 0..=MAX_REFERENCE_DEPTH + 1 {
            let next = format!("#/n{}", index + 1);
            root.insert(format!("n{index}"), json!({ "$ref": next }));
        }
        let root = serde_json::Value::Object(root);
        let mut resolver = Resolver::new(&root);
        assert!(resolver.resolve(&root["n0"]).is_none());
    }
}
