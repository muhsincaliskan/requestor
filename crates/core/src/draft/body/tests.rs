use super::{BodyDraft, BodyKind, RawLanguage};
use crate::collection::{Body, BodyMode, Param};
use crate::draft::KeyValueRow;

#[test]
fn an_untouched_body_round_trips_unchanged() {
    let body: Body = serde_json::from_str(
        r#"{"mode":"raw","raw":"{\"a\":1}","options":{"raw":{"language":"json"},"x":1},"x-extra":true}"#,
    )
    .unwrap();
    assert_eq!(BodyDraft::from_body(Some(&body)).to_body(), Some(body));
}

#[test]
fn switching_mode_keeps_the_other_modes_content() {
    let mut draft = BodyDraft::from_body(Some(&Body::raw("hello", Some("json"))));
    assert_eq!(draft.language, RawLanguage::Json);
    draft.kind = BodyKind::Urlencoded;
    draft.urlencoded.push(KeyValueRow::new("a", "1"));
    let body = draft.to_body().unwrap();
    assert_eq!(body.mode, Some(BodyMode::Urlencoded));
    assert_eq!(body.raw.as_deref(), Some("hello"));
    assert_eq!(body.urlencoded.unwrap().len(), 1);
}

#[test]
fn choosing_none_disables_a_body_that_has_content_and_omits_an_empty_one() {
    let mut draft = BodyDraft::from_body(Some(&Body::urlencoded(vec![Param::text("a", "1")])));
    draft.kind = BodyKind::None;
    let body = draft.to_body().unwrap();
    assert!(!body.is_enabled());
    assert_eq!(BodyDraft::from_body(Some(&body)).kind, BodyKind::None);
    assert!(BodyDraft::default().to_body().is_none());
}

#[test]
fn file_and_graphql_drafts_write_the_postman_shapes() {
    let draft = BodyDraft {
        kind: BodyKind::File,
        file_path: "/tmp/a.bin".to_string(),
        graphql_query: "{ me }".to_string(),
        ..BodyDraft::default()
    };
    let body = draft.to_body().unwrap();
    assert_eq!(body.file_source(), Some("/tmp/a.bin"));
    assert_eq!(body.graphql.unwrap()["query"], "{ me }");
}

#[test]
fn active_count_follows_the_selected_kind() {
    let mut draft = BodyDraft {
        raw: "x".to_string(),
        urlencoded: vec![KeyValueRow::new("a", "1"), KeyValueRow::default()],
        ..BodyDraft::default()
    };
    assert_eq!(draft.active_count(), 0);
    draft.kind = BodyKind::Raw;
    assert_eq!(draft.active_count(), 1);
    draft.kind = BodyKind::Urlencoded;
    assert_eq!(draft.active_count(), 1);
}
