use super::{AuthDraft, AuthKind};
use crate::collection::Auth;

#[test]
fn missing_auth_means_inherit_and_stays_missing() {
    let draft = AuthDraft::from_auth(None);
    assert_eq!(draft.kind, AuthKind::Inherit);
    assert!(draft.to_auth().is_none());
}

#[test]
fn untouched_and_unsupported_auth_is_preserved_exactly() {
    let oauth: Auth = serde_json::from_str(
        r#"{"type":"oauth2","oauth2":[{"key":"accessToken","value":"t","type":"string"}]}"#,
    )
    .unwrap();
    let draft = AuthDraft::from_auth(Some(&oauth));
    assert_eq!(draft.kind, AuthKind::Unsupported);
    assert_eq!(draft.unsupported_kind(), Some("oauth2"));
    assert_eq!(draft.to_auth(), Some(oauth));
}

const MIXED_AUTH: &str = r#"{"type":"bearer","bearer":[{"key":"token","value":"old","type":"string","x-attr":1}],
    "oauth2":[{"key":"accessToken","value":"t"}]}"#;

#[test]
fn editing_a_token_keeps_other_sections_and_attribute_extras() {
    let source: Auth = serde_json::from_str(MIXED_AUTH).unwrap();
    let mut draft = AuthDraft::from_auth(Some(&source));
    assert_eq!(draft.to_auth(), Some(source));
    draft.token = "new".to_string();
    let json = serde_json::to_value(draft.to_auth().unwrap()).unwrap();
    assert_eq!(json["bearer"][0]["value"], "new");
    assert_eq!(json["bearer"][0]["x-attr"], 1);
    assert_eq!(json["oauth2"][0]["key"], "accessToken");
}

#[test]
fn switching_to_a_kind_with_an_existing_section_reuses_it() {
    let source: Auth = serde_json::from_str(
        r#"{"type":"none","basic":[{"key":"username","value":"u","x-attr":2}]}"#,
    )
    .unwrap();
    let mut draft = AuthDraft::from_auth(Some(&source));
    draft.kind = AuthKind::Basic;
    draft.password = "p".to_string();
    let json = serde_json::to_value(draft.to_auth().unwrap()).unwrap();
    assert_eq!(json["type"], "basic");
    assert_eq!(json["basic"][0]["x-attr"], 2);
    assert_eq!(json["basic"][0]["value"], "u");
    assert_eq!(json["basic"][1]["value"], "p");
}

#[test]
fn inactive_sections_show_their_own_values() {
    let source: Auth = serde_json::from_str(
        r#"{"type":"bearer","bearer":[{"key":"token","value":"t"}],"basic":[{"key":"username","value":"ada"},{"key":"password","value":"pw"}]}"#,
    )
    .unwrap();
    let draft = AuthDraft::from_auth(Some(&source));
    assert_eq!(draft.token, "t");
    assert_eq!(draft.username, "ada");
    assert_eq!(draft.password, "pw");
}

#[test]
fn edits_produce_the_postman_shape_for_each_kind() {
    let mut draft = AuthDraft::from_auth(Some(&Auth::bearer("old")));
    draft.token = "new".to_string();
    assert_eq!(draft.to_auth().unwrap().value_of("token"), Some("new"));
    draft.kind = AuthKind::ApiKey;
    draft.api_key_name = "X-Key".to_string();
    draft.api_key_value = "v".to_string();
    draft.api_key_in_query = true;
    let api_key = draft.to_auth().unwrap();
    assert_eq!(api_key.kind, "apikey");
    assert_eq!(api_key.value_of("in"), Some("query"));
    draft.kind = AuthKind::None;
    assert!(draft.to_auth().unwrap().is_none());
}

#[test]
fn switching_kind_or_to_none_keeps_every_other_section() {
    let source: Auth = serde_json::from_str(MIXED_AUTH).unwrap();
    let mut draft = AuthDraft::from_auth(Some(&source));
    draft.kind = AuthKind::Basic;
    draft.username = "u".to_string();
    let basic = serde_json::to_value(draft.to_auth().unwrap()).unwrap();
    assert_eq!(basic["type"], "basic");
    assert_eq!(basic["bearer"][0]["x-attr"], 1);
    assert_eq!(basic["oauth2"][0]["key"], "accessToken");
    draft.kind = AuthKind::None;
    let none = serde_json::to_value(draft.to_auth().unwrap()).unwrap();
    assert_eq!(none["type"], "noauth");
    assert_eq!(none["bearer"][0]["value"], "old");
    assert_eq!(none["oauth2"][0]["key"], "accessToken");
}
