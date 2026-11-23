use serde_json::json;

use super::{SidebarOrder, SidebarSort};
use crate::collection::Item;

fn item(name: &str, folder: bool) -> Item {
    let value = if folder {
        json!({ "name": name, "item": [] })
    } else {
        json!({ "name": name, "request": { "method": "GET", "url": "https://example.com" } })
    };
    serde_json::from_value(value).unwrap()
}

fn names(items: &[&Item]) -> Vec<String> {
    items
        .iter()
        .map(|item| item.display_name().to_string())
        .collect()
}

fn sample() -> Vec<Item> {
    vec![
        item("delta", false),
        item("Bravo", true),
        item("alpha", false),
        item("Charlie", true),
    ]
}

#[test]
fn manual_order_keeps_the_saved_order() {
    let items = sample();
    let arranged = SidebarOrder::default().arrange_items(&items);
    assert_eq!(names(&arranged), ["delta", "Bravo", "alpha", "Charlie"]);
    assert!(SidebarOrder::default().allows_dragging());
    assert!(SidebarOrder::default().is_manual());
}

#[test]
fn names_sort_without_regard_to_case_in_both_directions() {
    let items = sample();
    let ascending = SidebarOrder {
        sort: SidebarSort::NameAscending,
        folders_first: false,
    };
    assert_eq!(
        names(&ascending.arrange_items(&items)),
        ["alpha", "Bravo", "Charlie", "delta"]
    );
    assert!(!ascending.allows_dragging());
    let descending = SidebarOrder {
        sort: SidebarSort::NameDescending,
        folders_first: false,
    };
    assert_eq!(
        names(&descending.arrange_items(&items)),
        ["delta", "Charlie", "Bravo", "alpha"]
    );
}

#[test]
fn folders_first_groups_folders_and_keeps_order_within_groups() {
    let items = sample();
    let manual = SidebarOrder {
        sort: SidebarSort::Manual,
        folders_first: true,
    };
    assert_eq!(
        names(&manual.arrange_items(&items)),
        ["Bravo", "Charlie", "delta", "alpha"]
    );
    assert!(manual.allows_dragging());
    let sorted = SidebarOrder {
        sort: SidebarSort::NameAscending,
        folders_first: true,
    };
    assert_eq!(
        names(&sorted.arrange_items(&items)),
        ["Bravo", "Charlie", "alpha", "delta"]
    );
}

#[test]
fn unknown_sort_names_fall_back_to_manual() {
    let order: SidebarOrder =
        serde_json::from_value(json!({ "sort": "recent", "folders_first": true })).unwrap();
    assert_eq!(order.sort, SidebarSort::Manual);
    assert!(order.folders_first);
}
