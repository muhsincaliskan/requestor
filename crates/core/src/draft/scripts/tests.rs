use crate::collection::Event;
use crate::draft::ScriptsDraft;
use crate::script::Phase;

fn events(json: &str) -> Option<Vec<Event>> {
    serde_json::from_str(json).unwrap()
}

#[test]
fn unchanged_foreign_scripts_are_written_back_untouched() {
    let original = events(r#"[{"listen":"test","script":{"exec":["console.log(1);"]}}]"#);
    let draft = ScriptsDraft::from_events(original.as_ref());
    assert!(draft.after.unconverted().is_some());
    assert!(draft.after.runnable().is_none());
    let mut stored = original.clone();
    draft.write_into(&mut stored);
    assert_eq!(stored, original);
}

#[test]
fn edited_scripts_are_written_as_javascript_and_committed() {
    let mut draft = ScriptsDraft::default();
    draft.before.text = "set id to new uuid in request".to_string();
    draft.after.text = "expect status is 200".to_string();
    let mut stored = None;
    draft.write_into(&mut stored);
    let reloaded = ScriptsDraft::from_events(stored.as_ref());
    assert_eq!(reloaded.after.text, "expect status is 200");
    assert_eq!(reloaded.before.phase(), Phase::BeforeRequest);
    assert_eq!(draft.committed(), draft.committed().committed());
    assert_eq!(draft.active_count(), 2);
    let mut draft = draft.committed();
    draft.after.text.clear();
    draft.write_into(&mut stored);
    assert_eq!(stored.unwrap().len(), 1);
}

#[test]
fn broken_text_keeps_the_saved_script_running() {
    let saved = events(
        r#"[{"listen":"test","script":{"exec":["pm.test(\"Status is 200\", () => pm.response.to.have.status(200));"]}}]"#,
    );
    let mut draft = ScriptsDraft::from_events(saved.as_ref());
    draft.after.text = "expect status iz 200".to_string();
    assert!(draft.after.parsed().is_err());
    assert!(draft.after.runnable().is_some());
    let mut stored = saved.clone();
    draft.write_into(&mut stored);
    assert_eq!(stored, saved);
}

#[test]
fn javascript_converts_both_ways_only_when_it_can() {
    let mut draft = ScriptsDraft::default();
    draft.after.text = "expect status is 201".to_string();
    assert!(draft.after.javascript_is_pending());
    draft.after.convert_to_javascript().unwrap();
    assert!(!draft.after.javascript_is_pending());
    draft.after.javascript = "pm.environment.set(\"a\", pm.response.json().a);".to_string();
    assert!(draft.after.javascript_is_pending());
    draft.after.convert_to_script().unwrap();
    assert_eq!(draft.after.text, "save json a as a");
    draft.after.javascript = "eval(x);".to_string();
    assert!(draft.after.convert_to_script().is_err());
    assert_eq!(draft.after.text, "save json a as a");
}
