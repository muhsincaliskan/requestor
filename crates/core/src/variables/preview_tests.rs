use super::{TextSegments, VariablePreview, VariableScope};
use crate::environment::Environment;

fn scope() -> VariableScope {
    let environment = Environment::from_json(
        r#"{"name":"E","values":[
            {"key":"base","value":"https://{{host}}","type":"default"},
            {"key":"host","value":"api.example.test","type":"default"},
            {"key":"token","value":"tok-1","type":"secret"},
            {"key":"header","value":"Bearer {{token}}","type":"default"}]}"#,
    )
    .unwrap();
    VariableScope::layered(&[], &[], Some(&environment), &[])
}

#[test]
fn preview_shows_the_resolved_value() {
    assert_eq!(
        scope().preview("base"),
        VariablePreview::Value("https://api.example.test".to_string())
    );
}

#[test]
fn preview_hides_secrets_and_values_that_embed_them() {
    assert_eq!(scope().preview("token"), VariablePreview::Secret);
    assert_eq!(scope().preview("header"), VariablePreview::Secret);
}

#[test]
fn preview_reports_undefined_names() {
    assert_eq!(scope().preview("missing"), VariablePreview::Undefined);
}

#[test]
fn placeholder_at_finds_the_name_under_a_character_index() {
    let text = "ğ/{{base}}/x";
    assert_eq!(TextSegments::placeholder_at(text, 1), None);
    assert_eq!(TextSegments::placeholder_at(text, 2), Some("base"));
    assert_eq!(TextSegments::placeholder_at(text, 6), Some("base"));
    assert_eq!(TextSegments::placeholder_at(text, 10), Some("base"));
    assert_eq!(TextSegments::placeholder_at(text, 11), None);
}
