use crate::collection::{Collection, Item, Request, Url};

fn sample() -> (Collection, String) {
    let mut collection = Collection::new("Sample");
    let request = collection
        .add_item(
            None,
            Item::for_request(
                "list".to_string(),
                Request::new("GET", Url::Raw("https://x.test/list".to_string())),
            ),
        )
        .unwrap();
    collection.ensure_ids();
    (collection, request)
}

#[test]
fn ensure_ids_names_the_collection_and_every_item_once() {
    let (mut collection, request) = sample();
    let collection_id = collection.id().to_string();
    assert!(!collection_id.is_empty());
    collection.ensure_ids();
    assert_eq!(collection.id(), collection_id);
    assert!(collection.find_item(&request).is_some());
}

#[test]
fn repeated_item_ids_are_replaced_so_every_item_can_be_found() {
    let mut collection: Collection = serde_json::from_str(
        r#"{"info":{"name":"Twins","schema":"s"},"item":[
            {"id":"same","name":"first","request":{"method":"GET","url":"https://a.test"}},
            {"id":"same","name":"second","request":{"method":"GET","url":"https://b.test"}}]}"#,
    )
    .unwrap();
    collection.ensure_ids();
    let ids: Vec<&str> = collection
        .item
        .iter()
        .filter_map(|item| item.id.as_deref())
        .collect();
    assert_eq!(ids[0], "same");
    assert_ne!(ids[1], "same");
    assert_eq!(
        collection.find_item(ids[1]).unwrap().display_name(),
        "second"
    );
}

#[test]
fn an_added_item_never_reuses_an_id_already_in_the_collection() {
    let mut collection = Collection::new("Sample");
    let mut first = Item::folder("a".to_string(), Vec::new());
    first.id = Some("fixed".to_string());
    let mut second = Item::folder("b".to_string(), Vec::new());
    second.id = Some("fixed".to_string());
    assert_eq!(collection.add_item(None, first).as_deref(), Some("fixed"));
    assert_ne!(collection.add_item(None, second).as_deref(), Some("fixed"));
}
