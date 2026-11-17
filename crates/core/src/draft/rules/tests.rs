use serde_json::json;

use super::{CheckKind, CheckRow, ExtractKind, ExtractRow, RulesDraft};
use crate::checks::{Check, JsonPath, VariableTarget};
use crate::collection::Event;

fn row(kind: CheckKind, subject: &str, expected: &str) -> CheckRow {
    CheckRow {
        kind,
        subject: subject.to_string(),
        expected: expected.to_string(),
    }
}

#[test]
fn rows_become_checks_only_when_complete() {
    assert_eq!(
        row(CheckKind::Status, "", "201").to_check(),
        Some(Check::Status(201))
    );
    assert_eq!(row(CheckKind::Status, "", "abc").to_check(), None);
    assert_eq!(row(CheckKind::HeaderContains, "", "json").to_check(), None);
    assert_eq!(row(CheckKind::JsonEquals, "a b", "1").to_check(), None);
    assert_eq!(
        row(CheckKind::JsonEquals, "user.name", "ada").to_check(),
        Some(Check::JsonEquals {
            path: JsonPath::parse("user.name").unwrap(),
            expected: json!("ada"),
        })
    );
}

#[test]
fn expected_json_values_keep_their_type_through_the_row() {
    for expected in [
        json!(5),
        json!("5"),
        json!("ada"),
        json!(true),
        json!({"a": 1}),
    ] {
        let check = Check::JsonEquals {
            path: JsonPath::parse("x").unwrap(),
            expected,
        };
        assert_eq!(CheckRow::from_check(&check).to_check(), Some(check));
    }
}

#[test]
fn extract_rows_need_a_variable_and_a_source() {
    let mut extract = ExtractRow {
        variable: "token".to_string(),
        kind: ExtractKind::Header,
        source: String::new(),
        target: VariableTarget::Collection,
    };
    assert_eq!(extract.to_extraction(), None);
    extract.source = "X-Token".to_string();
    let extraction = extract.to_extraction().unwrap();
    assert_eq!(ExtractRow::from_extraction(&extraction), extract);
}

#[test]
fn custom_scripts_are_never_overwritten() {
    let mut events: Option<Vec<Event>> =
        serde_json::from_str(r#"[{"listen":"test","script":{"exec":["console.log(1);"]}}]"#)
            .unwrap();
    let original = events.clone();
    let draft = RulesDraft::from_events(events.as_ref());
    assert!(draft.is_custom_script());
    draft.write_into(&mut events);
    assert_eq!(events, original);
}

#[test]
fn drafts_write_their_complete_rows_and_read_them_back() {
    let mut draft = RulesDraft::default();
    draft.checks.push(row(CheckKind::Status, "", "200"));
    draft.checks.push(row(CheckKind::BodyContains, "", ""));
    let mut events = None;
    draft.write_into(&mut events);
    let reread = RulesDraft::from_events(events.as_ref());
    assert_eq!(reread.checks, vec![row(CheckKind::Status, "", "200")]);
    assert_eq!(draft.active_count(), 1);
}
