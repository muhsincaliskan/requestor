use requestor_core::collection::{Collection, Item, Request, RequestSpec, Url};
use requestor_core::import::{import_text, ImportError, Imported, ImportedCollection};

const PETSTORE: &str = include_str!("fixtures/petstore_openapi.json");

fn import(text: &str) -> ImportedCollection {
    match import_text(text).unwrap() {
        Imported::Collection(imported) => imported,
        other => panic!("expected a collection, got {other:?}"),
    }
}

fn find<'a>(items: &'a [Item], name: &str) -> &'a Item {
    items
        .iter()
        .flat_map(|item| std::iter::once(item).chain(item.item.iter().flatten()))
        .find(|item| item.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no item named {name}"))
}

fn request_of(item: &Item) -> Request {
    match item.request.as_ref().unwrap() {
        RequestSpec::Full(request) => (**request).clone(),
        RequestSpec::Url(_) => panic!("expected a full request"),
    }
}

#[test]
fn builds_folders_names_and_the_base_url_variable() {
    let imported = import(PETSTORE);
    let collection = &imported.collection;
    assert_eq!(collection.info.name, "Petstore");
    assert_eq!(
        collection.info.description.as_ref().unwrap().content(),
        Some("A sample pet store")
    );
    let variable = &collection.variable.as_ref().unwrap()[0];
    assert_eq!(variable.key.as_deref(), Some("baseUrl"));
    assert_eq!(
        variable.text_value().unwrap(),
        "https://api.petstore.test/v1"
    );
    let folders: Vec<_> = collection
        .item
        .iter()
        .filter(|item| item.is_folder())
        .map(|item| item.name.clone().unwrap())
        .collect();
    assert_eq!(folders, ["pets", "stores"]);
    assert_eq!(collection.request_count(), 7);
    find(&collection.item, "addPet");
    find(&collection.item, "GET /health");
}

#[test]
fn merges_parameters_and_marks_optional_ones_disabled() {
    let imported = import(PETSTORE);
    let list = request_of(find(&imported.collection.item, "List pets"));
    assert_eq!(list.url.unwrap().to_text(), "{{baseUrl}}/pets?limit=25");
    let header = &list.header.unwrap()[0];
    assert_eq!(header.key.as_deref(), Some("X-Request-Id"));
    assert_eq!(header.disabled, Some(true));
    assert!(list.auth.unwrap().is_none());

    let get = request_of(find(&imported.collection.item, "Get a pet"));
    let url = get.url.unwrap();
    assert_eq!(url.to_text(), "{{baseUrl}}/pets/:petId");
    let Url::Parts(parts) = url else {
        panic!("expected url parts");
    };
    let variable = &parts.variable.unwrap()[0];
    assert_eq!(variable.key.as_deref(), Some("petId"));
    assert_eq!(variable.text_value().unwrap(), "42");
}

#[test]
fn bodies_come_from_schemas_examples_and_forms() {
    let imported = import(PETSTORE);
    let add = request_of(find(&imported.collection.item, "addPet"));
    assert_eq!(add.header_value("Content-Type"), Some("application/json"));
    let raw: serde_json::Value =
        serde_json::from_str(add.body.unwrap().raw.as_deref().unwrap()).unwrap();
    assert_eq!(
        raw,
        serde_json::json!({
            "name": "string", "status": "available",
            "birthday": "2024-01-01", "toys": ["string"]
        })
    );
    let put = request_of(find(&imported.collection.item, "Replace a pet"));
    let raw: serde_json::Value =
        serde_json::from_str(put.body.unwrap().raw.as_deref().unwrap()).unwrap();
    assert_eq!(raw, serde_json::json!({"name": "Rex", "tag": "dog"}));
    let upload = request_of(find(&imported.collection.item, "Upload a photo"));
    let fields = upload.body.unwrap().formdata.unwrap();
    let photo = fields
        .iter()
        .find(|field| field.key.as_deref() == Some("photo"))
        .unwrap();
    assert_eq!(photo.kind.as_deref(), Some("file"));
}

#[test]
fn cyclic_schemas_terminate_with_a_finite_body() {
    let imported = import(PETSTORE);
    let store = request_of(find(&imported.collection.item, "Create a store"));
    let raw: serde_json::Value =
        serde_json::from_str(store.body.unwrap().raw.as_deref().unwrap()).unwrap();
    assert_eq!(
        raw,
        serde_json::json!({"name": "string", "owner": {"name": "string", "employer": null}})
    );
}

#[test]
fn security_schemes_map_to_auth_and_unsupported_ones_warn() {
    let imported = import(PETSTORE);
    let collection = &imported.collection;
    assert_eq!(
        collection.auth.as_ref().unwrap().value_of("token"),
        Some("{{bearerToken}}")
    );
    let add = request_of(find(&collection.item, "addPet"));
    assert_eq!(
        add.auth.as_ref().unwrap().value_of("key"),
        Some("X-Api-Key")
    );
    let store = request_of(find(&collection.item, "Create a store"));
    assert_eq!(
        store.auth.unwrap().value_of("username"),
        Some("{{username}}")
    );
    let put = request_of(find(&collection.item, "Replace a pet"));
    assert!(put.auth.unwrap().is_none());
    let warnings = imported.warnings.join("\n");
    for expected in ["oauth", "cookie parameter session", "external reference"] {
        assert!(warnings.contains(expected), "{expected} in {warnings}");
    }
}

#[test]
fn the_imported_collection_round_trips_as_postman_json() {
    let imported = import(PETSTORE);
    let json = imported.collection.to_json_pretty().unwrap();
    assert!(json.contains("collection/v2.1.0"));
    assert_eq!(Collection::from_json(&json).unwrap(), imported.collection);
}

#[test]
fn swagger_two_and_yaml_are_rejected_with_a_conversion_hint() {
    let swagger = import_text(r#"{"swagger": "2.0", "info": {"title": "x"}}"#).unwrap_err();
    assert!(matches!(swagger, ImportError::UnsupportedFormat(_)));
    assert!(swagger.to_string().contains("convert to OpenAPI 3 JSON"));
    let yaml = import_text("openapi: 3.0.0\ninfo:\n  title: x").unwrap_err();
    assert!(yaml.to_string().contains("convert to OpenAPI 3 JSON"));
}

#[test]
fn the_size_cap_applies_before_parsing() {
    let padding = "x".repeat(10 * 1024 * 1024);
    let broken = format!(r#"{{"openapi": "3.0.0", "x-padding": "{padding}"#);
    let error = import_text(&broken).unwrap_err();
    assert!(matches!(error, ImportError::UnsupportedFormat(_)));
}

#[test]
fn operations_with_only_unsupported_security_get_explicit_no_auth() {
    let document = serde_json::json!({
        "openapi": "3.0.0",
        "servers": [{"url": "https://x.test"}],
        "security": [{"key": []}],
        "components": {"securitySchemes": {
            "key": {"type": "apiKey", "in": "header", "name": "X-Key"},
            "oauth": {"type": "oauth2"}
        }},
        "paths": {"/a": {"get": {"summary": "A", "security": [{"oauth": []}]}}}
    });
    let imported = import(&document.to_string());
    let request = request_of(find(&imported.collection.item, "A"));
    assert!(request.auth.unwrap().is_none());
    assert!(imported
        .warnings
        .iter()
        .any(|warning| warning.contains("oauth")));
}

#[test]
fn path_items_may_be_local_references_and_external_ones_warn() {
    let document = serde_json::json!({
        "openapi": "3.0.0",
        "servers": [{"url": "https://x.test"}],
        "paths": {
            "/a": {"$ref": "#/components/pathItems/Shared"},
            "/b": {"$ref": "https://example.test/paths.json#/B"}
        },
        "components": {"pathItems": {"Shared": {"get": {"summary": "Shared"}}}}
    });
    let imported = import(&document.to_string());
    assert_eq!(imported.collection.request_count(), 1);
    find(&imported.collection.item, "Shared");
    assert!(imported
        .warnings
        .iter()
        .any(|warning| warning.contains("external reference")));
}

#[test]
fn oversized_documents_and_operation_floods_are_bounded() {
    let padding = "x".repeat(10 * 1024 * 1024);
    let huge = format!(r#"{{"openapi": "3.0.0", "x-padding": "{padding}"}}"#);
    assert!(import_text(&huge)
        .unwrap_err()
        .to_string()
        .contains("10 MB"));

    let paths: serde_json::Map<String, serde_json::Value> = (0..2001)
        .map(|index| (format!("/p{index}"), serde_json::json!({"get": {}})))
        .collect();
    let document = serde_json::json!({"openapi": "3.0.0", "paths": paths});
    let imported = import(&document.to_string());
    assert_eq!(imported.collection.request_count(), 2000);
    assert!(imported
        .warnings
        .iter()
        .any(|warning| warning.contains("2000")));
}
