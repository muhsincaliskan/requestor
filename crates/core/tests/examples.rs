use requestor_core::collection::{Example, Item, Request};
use requestor_core::header::Header;
use requestor_core::response::HttpResponse;
use serde_json::json;

fn item_with_postman_example() -> Item {
    serde_json::from_value(json!({
        "name": "Get user",
        "request": {"method": "GET", "url": "https://api.example.test/users/1"},
        "response": [
            {
                "name": "Found",
                "originalRequest": {"method": "GET", "url": "https://api.example.test/users/1"},
                "status": "OK",
                "code": 200,
                "_postman_previewlanguage": "json",
                "header": [{"key": "Content-Type", "value": "application/json"}],
                "body": "{\"id\":1}"
            },
            {"name": "Odd", "header": "not a list", "code": 500}
        ]
    }))
    .unwrap()
}

#[test]
fn imported_examples_are_listed_and_open_as_responses() {
    let item = item_with_postman_example();
    let names: Vec<_> = item
        .example_summaries()
        .map(|summary| {
            (
                summary.index,
                summary.display_name().to_string(),
                summary.code,
            )
        })
        .collect();
    assert_eq!(
        names,
        vec![
            (0, "Found".to_string(), Some(200)),
            (1, "Odd".to_string(), Some(500))
        ]
    );
    let example = item.example(0).unwrap();
    let response = example.to_response();
    assert_eq!((response.status, response.reason.as_str()), (200, "OK"));
    assert_eq!(response.headers[0].name, "Content-Type");
    assert_eq!(response.body, b"{\"id\":1}");
    assert!(example.request().is_some());
    assert!(item.example(1).is_none());
}

#[test]
fn unknown_example_fields_survive_a_round_trip() {
    let item = item_with_postman_example();
    let exported = serde_json::to_value(&item).unwrap();
    assert_eq!(exported["response"][0]["_postman_previewlanguage"], "json");
    assert_eq!(exported["response"][1]["header"], "not a list");
}

#[test]
fn saved_examples_are_added_and_removed() {
    let mut item = item_with_postman_example();
    let request: Request =
        serde_json::from_value(json!({"method": "POST", "url": "https://api.example.test/users"}))
            .unwrap();
    let response = HttpResponse {
        status: 201,
        reason: "Created".to_string(),
        version: "HTTP/1.1".to_string(),
        headers: vec![Header {
            name: "Location".to_string(),
            value: "/users/2".to_string(),
        }],
        body: b"{}".to_vec(),
        truncated: false,
        elapsed: std::time::Duration::from_millis(5),
        timing: None,
    };
    item.add_example(&Example::from_exchange(&request, &response));
    let saved = item.example(2).unwrap();
    assert_eq!(saved.display_name(), "201 Created");
    assert_eq!(saved.to_response().headers[0].value, "/users/2");
    assert!(item.remove_example(0));
    assert!(item.remove_example(0));
    assert!(item.remove_example(0));
    assert!(item.response.is_none());
    assert!(!item.remove_example(0));
}
