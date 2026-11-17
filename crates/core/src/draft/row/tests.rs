use super::KeyValueRow;
use crate::collection::Param;

#[test]
fn unknown_param_fields_survive_an_edit() {
    let param: Param =
        serde_json::from_str(r#"{"key":"a","value":"1","x-custom":true,"description":"d"}"#)
            .unwrap();
    let mut row = KeyValueRow::from_param(&param);
    row.value = "2".to_string();
    row.enabled = false;
    let edited = row.to_param();
    assert_eq!(edited.value.as_deref(), Some("2"));
    assert_eq!(edited.disabled, Some(true));
    assert!(edited.extra.contains_key("x-custom"));
    assert_eq!(edited.description, param.description);
}

#[test]
fn file_rows_keep_the_path_in_src() {
    let row = KeyValueRow::from_param(&Param::file("doc", "/tmp/a.pdf"));
    assert!(row.is_file);
    assert_eq!(row.value, "/tmp/a.pdf");
    let param = row.to_param();
    assert_eq!(param.kind.as_deref(), Some("file"));
    assert!(param.value.is_none());
}

#[test]
fn turning_a_file_row_into_text_drops_the_source_path() {
    let mut row = KeyValueRow::from_param(&Param::file("doc", "/tmp/a.pdf"));
    row.is_file = false;
    row.value = "hello".to_string();
    let param = row.to_param();
    assert_eq!(param.kind.as_deref(), Some("text"));
    assert!(param.src.is_none());
    assert_eq!(param.value.as_deref(), Some("hello"));
}

#[test]
fn valueless_query_pairs_stay_valueless() {
    assert_eq!(
        KeyValueRow::from_param(&Param::from_query_pair("flag")).query_pair(),
        "flag"
    );
    assert_eq!(KeyValueRow::new("a", "").query_pair(), "a=");
    assert_eq!(KeyValueRow::new("a", "1").query_pair(), "a=1");
}

#[test]
fn a_typed_row_joins_the_table_and_a_blank_one_stays_pending() {
    let mut rows = vec![KeyValueRow::new("a", "1")];
    let pending = KeyValueRow::default().commit_into(&mut rows);
    assert!(pending.is_some());
    assert_eq!(rows.len(), 1);
    assert!(KeyValueRow::new("b", "").commit_into(&mut rows).is_none());
    assert_eq!(rows.len(), 2);
}

#[test]
fn active_count_skips_disabled_and_blank_rows() {
    let mut disabled = KeyValueRow::new("off", "1");
    disabled.enabled = false;
    let rows = vec![KeyValueRow::new("a", "1"), disabled, KeyValueRow::default()];
    assert_eq!(KeyValueRow::active_count(&rows), 1);
}

#[test]
fn several_files_survive_an_edit_elsewhere_in_the_row() {
    let param: Param =
        serde_json::from_str(r#"{"key":"f","type":"file","src":["a.png","b.png"]}"#).unwrap();
    let mut row = KeyValueRow::from_param(&param);
    row.description = "photos".to_string();
    let json = serde_json::to_value(row.to_param()).unwrap();
    assert_eq!(json["src"], serde_json::json!(["a.png", "b.png"]));
}

#[test]
fn an_explicit_enabled_flag_survives_an_edit() {
    let param: Param = serde_json::from_str(r#"{"key":"a","value":"1","disabled":false}"#).unwrap();
    let mut row = KeyValueRow::from_param(&param);
    row.value = "2".to_string();
    assert_eq!(row.to_param().disabled, Some(false));
}
