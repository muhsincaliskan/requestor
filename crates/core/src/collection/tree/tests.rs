use crate::collection::{Auth, Collection, Item, Request, Url, Variable};

fn request_item(name: &str) -> Item {
    Item::for_request(
        name.to_string(),
        Request::new("GET", Url::Raw(format!("https://x.test/{name}"))),
    )
}

fn sample() -> (Collection, String, String, String) {
    let mut collection = Collection::new("Sample");
    let folder = collection
        .add_item(None, Item::folder("Users".to_string(), Vec::new()))
        .unwrap();
    let inner = collection
        .add_item(Some(&folder), Item::folder("Admin".to_string(), Vec::new()))
        .unwrap();
    let request = collection
        .add_item(Some(&inner), request_item("list"))
        .unwrap();
    collection.ensure_ids();
    (collection, folder, inner, request)
}

#[test]
fn ensure_ids_names_the_collection_and_every_item_once() {
    let (mut collection, _, _, request) = sample();
    let collection_id = collection.id().to_string();
    assert!(!collection_id.is_empty());
    collection.ensure_ids();
    assert_eq!(collection.id(), collection_id);
    assert!(collection.find_item(&request).is_some());
}

#[test]
fn items_are_found_and_edited_by_id_at_any_depth() {
    let (mut collection, _, _, request) = sample();
    collection.find_item_mut(&request).unwrap().name = Some("renamed".to_string());
    assert_eq!(
        collection.find_item(&request).unwrap().display_name(),
        "renamed"
    );
    assert!(collection.find_item("missing").is_none());
}

#[test]
fn adding_into_a_request_or_a_missing_parent_is_refused() {
    let (mut collection, _, _, request) = sample();
    assert!(collection
        .add_item(Some(&request), request_item("x"))
        .is_none());
    assert!(collection
        .add_item(Some("missing"), request_item("x"))
        .is_none());
    assert_eq!(collection.request_count(), 1);
}

#[test]
fn removing_a_folder_removes_its_subtree() {
    let (mut collection, folder, _, request) = sample();
    let removed = collection.remove_item(&folder).unwrap();
    assert_eq!(removed.request_count(), 1);
    assert!(collection.find_item(&request).is_none());
    assert!(collection.remove_item(&folder).is_none());
}

#[test]
fn moving_keeps_the_item_and_refuses_cycles() {
    let (mut collection, folder, inner, request) = sample();
    assert!(!collection.move_item(&folder, Some(&inner), 0));
    assert!(!collection.move_item(&folder, Some(&folder), 0));
    assert!(!collection.move_item(&request, Some("missing"), 0));
    assert!(collection.find_item(&request).is_some());
    assert!(collection.move_item(&request, None, 0));
    assert!(collection.item[0].has_id(&request));
    assert!(collection.move_item(&request, Some(&folder), 99));
    assert!(collection.find_item(&folder).unwrap().children()[1].has_id(&request));
}

#[test]
fn duplicates_sit_after_the_original_with_fresh_ids() {
    let (mut collection, folder, inner, _) = sample();
    let copy = collection.duplicate_item(&inner).unwrap();
    assert_ne!(copy, inner);
    let siblings = collection.find_item(&folder).unwrap().children();
    assert_eq!(siblings.len(), 2);
    assert!(siblings[1].has_id(&copy));
    assert_eq!(siblings[1].display_name(), "Admin Copy");
    assert_ne!(siblings[1].children()[0].id, siblings[0].children()[0].id);
    assert_eq!(collection.request_count(), 2);
}

#[test]
fn auth_is_inherited_from_the_nearest_folder_then_the_collection() {
    let (mut collection, folder, inner, request) = sample();
    assert!(collection.inherited_auth(&request).is_none());
    collection.auth = Some(Auth::bearer("collection"));
    assert_eq!(
        collection
            .inherited_auth(&request)
            .unwrap()
            .value_of("token"),
        Some("collection")
    );
    collection.find_item_mut(&folder).unwrap().auth = Some(Auth::bearer("outer"));
    collection.find_item_mut(&inner).unwrap().auth = Some(Auth::none());
    assert!(collection.inherited_auth(&request).unwrap().is_none());
    assert_eq!(
        collection.inherited_auth(&inner).unwrap().value_of("token"),
        Some("outer")
    );
}

#[test]
fn inherited_variables_run_from_the_collection_inwards() {
    let (mut collection, folder, inner, request) = sample();
    collection.variable = Some(vec![Variable::text("host", "collection")]);
    collection.find_item_mut(&folder).unwrap().variable = Some(vec![Variable::text("a", "1")]);
    collection.find_item_mut(&inner).unwrap().variable = Some(vec![Variable::text("b", "2")]);
    let keys: Vec<_> = collection
        .inherited_variables(&request)
        .into_iter()
        .filter_map(|variable| variable.key.as_deref())
        .collect();
    assert_eq!(keys, ["host", "a", "b"]);
}
