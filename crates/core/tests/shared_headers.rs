use requestor_core::collection::{Collection, Item, Param, Request, Url};
use requestor_core::workspace::Workspace;

fn header_names(request: &Request) -> Vec<String> {
    request
        .header
        .iter()
        .flatten()
        .map(|header| {
            format!(
                "{}={}",
                header.key.clone().unwrap_or_default(),
                header.value.clone().unwrap_or_default()
            )
        })
        .collect()
}

fn sample() -> (Collection, String, String) {
    let mut collection = Collection::new("Shop");
    collection.set_shared_headers(&[
        Param::text("X-Tenant", "root"),
        Param::text("Accept", "*/*"),
    ]);
    let folder_id = collection
        .add_item(None, Item::folder("Orders".to_string(), Vec::new()))
        .unwrap();
    let mut folder_headers = vec![Param::text("X-Tenant", "orders")];
    let mut disabled = Param::text("X-Debug", "1");
    disabled.disabled = Some(true);
    folder_headers.push(disabled);
    collection
        .find_item_mut(&folder_id)
        .unwrap()
        .set_shared_headers(&folder_headers);
    let mut request = Request::new("GET", Url::Raw("https://shop.test/orders".to_string()));
    request.header = Some(vec![Param::text("accept", "application/json")]);
    let item_id = collection
        .add_item(
            Some(&folder_id),
            Item::for_request("List".to_string(), request),
        )
        .unwrap();
    (collection, folder_id, item_id)
}

#[test]
fn the_nearest_folder_wins_and_the_request_wins_over_both() {
    let (collection, _, item_id) = sample();
    let mut workspace = Workspace::default();
    let collection_id = workspace.add_collection(collection);
    let stored = workspace
        .collection(&collection_id)
        .and_then(|collection| collection.find_item(&item_id))
        .and_then(Item::full_request)
        .unwrap();
    let sent = workspace.sendable_request(Some(&collection_id), Some(&item_id), stored);
    assert_eq!(
        header_names(&sent),
        ["X-Tenant=orders", "accept=application/json"]
    );
}

#[test]
fn the_runner_applies_the_same_headers() {
    let (collection, _, _) = sample();
    let runnable = collection.runnable_requests();
    assert_eq!(
        header_names(&runnable[0].request),
        ["X-Tenant=orders", "accept=application/json"]
    );
}

#[test]
fn inherited_headers_name_their_owner_and_survive_a_round_trip() {
    let (collection, folder_id, item_id) = sample();
    let inherited = collection.inherited_headers(&item_id);
    let owners: Vec<&str> = inherited.iter().map(|entry| entry.owner.as_str()).collect();
    assert_eq!(owners, ["Shop", "Shop", "Orders", "Orders"]);
    let restored = Collection::from_json(&collection.to_json_pretty().unwrap()).unwrap();
    assert_eq!(
        restored
            .find_item(&folder_id)
            .unwrap()
            .shared_headers()
            .len(),
        2
    );
    let mut cleared = restored;
    cleared.set_shared_headers(&[]);
    assert!(!cleared.extra.contains_key("requestorHeaders"));
}

#[test]
fn the_auth_owner_is_the_nearest_folder_with_auth() {
    let (mut collection, folder_id, item_id) = sample();
    assert!(collection.auth_owner(&item_id).is_none());
    collection.auth = Some(requestor_core::collection::Auth::bearer("root"));
    assert_eq!(collection.auth_owner(&item_id).unwrap().1, "Shop");
    collection.find_item_mut(&folder_id).unwrap().auth =
        Some(requestor_core::collection::Auth::bearer("orders"));
    assert_eq!(collection.auth_owner(&item_id).unwrap().1, "Orders");
}

#[test]
fn effective_inherited_headers_skip_names_the_request_sets() {
    let (collection, _, item_id) = sample();
    let inherited = collection.inherited_headers(&item_id);
    let effective = requestor_core::collection::InheritedHeader::effective(&inherited, &["ACCEPT"]);
    let shown: Vec<(&str, &str)> = effective
        .iter()
        .map(|entry| (entry.header.key.as_deref().unwrap(), entry.owner.as_str()))
        .collect();
    assert_eq!(shown, [("X-Tenant", "Orders")]);
}
