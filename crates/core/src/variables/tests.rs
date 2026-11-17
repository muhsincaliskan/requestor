use super::{placeholder_names, TextSegment, TextSegments, VariableLayer, VariableScope};
use crate::collection::Variable;
use crate::environment::Environment;

fn scope() -> VariableScope {
    VariableScope::new()
        .with_layer(
            VariableLayer::default()
                .with("host", "global.example")
                .with("a", "1"),
        )
        .with_layer(VariableLayer::default().with("host", "env.example"))
}

fn scope_with(pairs: &[(&str, &str)]) -> VariableScope {
    let layer = pairs
        .iter()
        .fold(VariableLayer::default(), |layer, (key, value)| {
            layer.with(*key, *value)
        });
    VariableScope::new().with_layer(layer)
}

#[test]
fn later_layers_override_earlier_ones() {
    assert_eq!(
        scope().resolve("https://{{host}}/x").text,
        "https://env.example/x"
    );
    assert_eq!(scope().resolve("{{a}}").text, "1");
}

#[test]
fn unresolved_names_keep_their_original_spelling_and_are_reported_once() {
    let resolved = scope().resolve("{{missing}}/{{ missing }}/{{host}}");
    assert_eq!(resolved.text, "{{missing}}/{{ missing }}/env.example");
    assert_eq!(resolved.unresolved, vec!["missing".to_string()]);
}

#[test]
fn nested_variables_expand() {
    let nested = scope_with(&[("base", "https://{{host}}"), ("host", "h")]);
    assert_eq!(nested.resolve("{{base}}/p").text, "https://h/p");
}

#[test]
fn cycles_are_reported_and_left_unexpanded() {
    let resolved = scope_with(&[("a", "{{b}}"), ("b", "{{a}}")]).resolve("{{a}}");
    assert_eq!(resolved.text, "{{a}}");
    assert_eq!(resolved.circular, vec!["a".to_string()]);
    let selfish = scope_with(&[("a", "x{{a}}")]).resolve("{{a}}");
    assert_eq!(selfish.text, "x{{a}}");
    assert_eq!(selfish.circular, vec!["a".to_string()]);
}

#[test]
fn exponential_expansion_is_bounded() {
    let fanout = "{{c}}".repeat(8);
    let mut pairs: Vec<(String, String)> = (0..12)
        .map(|level| {
            (
                format!("v{level}"),
                format!("{{{{v{}}}}}", level + 1).repeat(8),
            )
        })
        .collect();
    pairs.push(("v12".to_string(), "x".to_string()));
    pairs.push(("c".to_string(), fanout));
    let layer = pairs
        .iter()
        .fold(VariableLayer::default(), |layer, (key, value)| {
            layer.with(key.as_str(), value.as_str())
        });
    let resolved = VariableScope::new().with_layer(layer).resolve("{{v0}}");
    assert!(resolved.limit_exceeded);
    assert!(resolved.text.len() < 16 * 1024 * 1024);
}

#[test]
fn placeholder_parsing_matches_postman_for_edge_cases() {
    let a = scope();
    assert_eq!(a.resolve("{{{a}}}").text, "{1}");
    assert_eq!(a.resolve("{{x {{a}}").text, "{{x 1");
    assert_eq!(a.resolve("{{}}").text, "{{}}");
    assert!(a.resolve("{{}}").unresolved.is_empty());
    assert_eq!(a.resolve("}} {{a}}").text, "}} 1");
    assert_eq!(a.resolve("a {{host").text, "a {{host");
}

#[test]
fn placeholder_names_lists_distinct_names() {
    assert_eq!(
        placeholder_names("{{b}} {{a}} {{b}} {{ }}"),
        vec!["a".to_string(), "b".to_string()]
    );
}

#[test]
fn layers_follow_collection_folder_environment_override_precedence() {
    let variable = |value: &str| -> Vec<Variable> {
        serde_json::from_str(&format!(r#"[{{"key":"v","value":"{value}"}}]"#)).unwrap()
    };
    let environment =
        Environment::from_json(r#"{"name":"E","values":[{"key":"v","value":"env"}]}"#).unwrap();
    let resolve = |scope: VariableScope| scope.resolve("{{v}}").text;
    let (collection, folder) = (variable("collection"), variable("folder"));
    assert_eq!(
        resolve(VariableScope::layered(&collection, &[], None, &[])),
        "collection"
    );
    assert_eq!(
        resolve(VariableScope::layered(&collection, &folder, None, &[])),
        "folder"
    );
    assert_eq!(
        resolve(VariableScope::layered(
            &collection,
            &folder,
            Some(&environment),
            &[]
        )),
        "env"
    );
    let overrides = [("v".to_string(), "cli".to_string())];
    assert_eq!(
        resolve(VariableScope::layered(
            &collection,
            &folder,
            Some(&environment),
            &overrides
        )),
        "cli"
    );
}

#[test]
fn secret_values_come_from_secret_typed_entries_only() {
    let environment = Environment::from_json(
        r#"{"name":"E","values":[
            {"key":"apiToken","value":"tok-1","type":"secret"},
            {"key":"nested","value":"x-{{apiToken}}","type":"secret"},
            {"key":"plain","value":"visible","type":"default"}]}"#,
    )
    .unwrap();
    let scope = VariableScope::layered(&[], &[], Some(&environment), &[]);
    let secrets = scope.secret_values();
    assert!(secrets.contains(&"tok-1".to_string()));
    assert!(secrets.contains(&"x-tok-1".to_string()));
    assert!(!secrets.contains(&"visible".to_string()));
}

#[test]
fn debug_output_hides_values() {
    let debug = format!("{:?}", scope_with(&[("token", "s3cret")]));
    assert!(debug.contains("token"));
    assert!(!debug.contains("s3cret"));
}

#[test]
fn names_are_sorted_and_deduplicated_across_layers() {
    let scope = scope_with(&[("b", "1"), ("a", "2")])
        .with_layer(VariableLayer::default().with("a", "3").with("c", "4"));
    assert_eq!(scope.names(), ["a", "b", "c"]);
    assert!(VariableScope::new().names().is_empty());
}

#[test]
fn segments_split_text_around_placeholders() {
    let segments: Vec<_> = TextSegments::new("{{base}}/pets/{{ id }}?x={{").collect();
    assert_eq!(
        segments,
        vec![
            TextSegment::Placeholder {
                literal: "{{base}}",
                name: "base"
            },
            TextSegment::Plain("/pets/"),
            TextSegment::Placeholder {
                literal: "{{ id }}",
                name: "id"
            },
            TextSegment::Plain("?x={{"),
        ]
    );
}

#[test]
fn segments_rebuild_the_original_text() {
    let text = "a{{}}b{{x}}{{y}}c}}";
    let rebuilt: String = TextSegments::new(text)
        .map(|s| s.text().to_string())
        .collect();
    assert_eq!(rebuilt, text);
    assert_eq!(TextSegments::new("").count(), 0);
}

#[test]
fn only_placeholders_with_known_names_match() {
    let names = vec!["base".to_string()];
    let mut segments = TextSegments::new("{{base}}{{other}}plain");
    assert!(segments.next().unwrap().is_known_in(&names));
    assert!(!segments.next().unwrap().is_known_in(&names));
    assert!(!segments.next().unwrap().is_known_in(&names));
}
