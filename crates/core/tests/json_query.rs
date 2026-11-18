use requestor_core::json_query::JsonQuery;
use requestor_core::json_tree::{JsonKey, JsonKind, JsonNode};

const STORE: &str = r#"{
  "store": {
    "book": [
      { "title": "Sayings", "price": 8.95, "tags": ["old"] },
      { "title": "Sword", "price": 12.99, "isbn": "0-553" },
      { "title": "Moby", "price": 8.99, "isbn": "0-395" },
      { "title": "Rings", "price": 22.99 }
    ],
    "bicycle": { "color": "red", "price": 19.95 }
  },
  "content-type": "json"
}"#;

fn select(query: &str) -> String {
    let document = JsonNode::parse(STORE).unwrap();
    let found = JsonQuery::parse(query).unwrap().select(&document);
    JsonNode::list_text(&found, false)
}

#[test]
fn names_indices_and_wildcards_select_values() {
    assert_eq!(select("$.store.book[0].title"), r#"["Sayings"]"#);
    assert_eq!(select("$['store']['bicycle'].color"), r#"["red"]"#);
    assert_eq!(select("$.store.book[-1].title"), r#"["Rings"]"#);
    assert_eq!(select("$.store.book[*].price"), "[8.95,12.99,8.99,22.99]");
    assert_eq!(select("$.store.bicycle.*"), r#"["red",19.95]"#);
    assert_eq!(select("$.content-type"), r#"["json"]"#);
}

#[test]
fn the_root_sign_is_optional() {
    assert_eq!(select("store.bicycle.color"), r#"["red"]"#);
    assert_eq!(select("[\"content-type\"]"), r#"["json"]"#);
    assert_eq!(select(""), select("$"));
}

#[test]
fn slices_and_unions_pick_several_elements() {
    let titles = |query: &str| select(&format!("$.store.book[{query}].title"));
    assert_eq!(titles("1:3"), r#"["Sword","Moby"]"#);
    assert_eq!(titles(":2"), r#"["Sayings","Sword"]"#);
    assert_eq!(titles("-2:"), r#"["Moby","Rings"]"#);
    assert_eq!(titles("::2"), r#"["Sayings","Moby"]"#);
    assert_eq!(titles("::-1"), r#"["Rings","Moby","Sword","Sayings"]"#);
    assert_eq!(titles("0,3"), r#"["Sayings","Rings"]"#);
}

#[test]
fn descendants_are_searched_in_document_order() {
    assert_eq!(select("$..price"), "[8.95,12.99,8.99,22.99,19.95]");
    assert_eq!(select("$..book[0].title"), r#"["Sayings"]"#);
}

#[test]
fn filters_compare_members_with_literals() {
    assert_eq!(
        select("$.store.book[?(@.price < 9)].title"),
        r#"["Sayings","Moby"]"#
    );
    assert_eq!(select("$.store.book[?@.isbn].title"), r#"["Sword","Moby"]"#);
    assert_eq!(
        select("$.store.book[?(@.title == 'Rings')].price"),
        "[22.99]"
    );
    assert_eq!(
        select("$.store.book[?(@.tags[0] != \"old\")].title"),
        r#"["Sword","Moby","Rings"]"#
    );
    assert_eq!(select("$..book[?(@.price >= 22.99)].title"), r#"["Rings"]"#);
}

#[test]
fn missing_values_select_nothing() {
    assert_eq!(select("$.store.car"), "[]");
    assert_eq!(select("$.store.book[9]"), "[]");
    assert_eq!(select("$.store.bicycle[0]"), "[]");
}

#[test]
fn malformed_queries_report_where_they_went_wrong() {
    for (query, position) in [
        ("$.store.", 9),
        ("$[1", 4),
        ("$.a[?(@.b == )]", 14),
        ("$x", 2),
    ] {
        let error = JsonQuery::parse(query).unwrap_err();
        assert_eq!(error.position, position, "{query}");
    }
}

#[test]
fn trees_keep_member_order_and_describe_their_children() {
    let document = JsonNode::parse(r#"{"z": 1, "a": [true, null, "x"]}"#).unwrap();
    assert_eq!(document.to_text(false), r#"{"z":1,"a":[true,null,"x"]}"#);
    assert_eq!(document.kind(), JsonKind::Object);
    assert_eq!(document.summary(), "{2}");
    let (key, list) = document.child(1).unwrap();
    assert_eq!(key, JsonKey::Name("a"));
    assert_eq!(list.summary(), "[3]");
    assert_eq!(
        list.child(2).map(|(key, node)| (key, node.summary())),
        Some((JsonKey::Index(2), "\"x\"".to_string()))
    );
    assert!(JsonNode::parse("{not json").is_none());
}
