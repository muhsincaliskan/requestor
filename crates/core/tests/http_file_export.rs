use serde_json::json;

#[test]
fn a_collection_exports_as_one_http_file() {
    let collection: requestor_core::collection::Collection = serde_json::from_value(json!({
        "info": {"name": "Users", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
        "variable": [
            {"key": "base", "value": "https://api.example.test"},
            {"key": "token", "value": "hidden", "type": "secret"}
        ],
        "item": [
            {"name": "Admin", "item": [
                {"name": "List", "request": {"method": "GET", "url": "{{base}}/users"}}
            ]},
            {"name": "Create", "request": {
                "method": "POST",
                "url": "{{base}}/users",
                "header": [{"key": "Authorization", "value": "Bearer {{token}}"}],
                "body": {"mode": "raw", "raw": "{\"name\":\"Ada\"}"}
            }}
        ]
    }))
    .unwrap();
    let file = collection.to_http_file(&requestor_core::settings::RequestSettings::default());
    assert!(
        file.starts_with("@base = https://api.example.test\n\n"),
        "{file}"
    );
    assert!(!file.contains("hidden"), "{file}");
    assert!(!file.contains("Postman-Token"), "{file}");
    assert!(
        file.contains("### Admin/List\nGET {{base}}/users\n"),
        "{file}"
    );
    assert!(file.contains("### Create\nPOST {{base}}/users\n"), "{file}");
    assert!(file.contains("Authorization: Bearer {{token}}"), "{file}");
    assert!(file.contains("{\"name\":\"Ada\"}"), "{file}");
}
