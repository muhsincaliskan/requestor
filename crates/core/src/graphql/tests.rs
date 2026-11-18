use serde_json::{json, Value};

use super::{GraphQlSchema, OperationKind, SchemaError, TypeRef};
use crate::draft::BodyDraft;
use crate::header::Header;
use crate::method::HttpMethod;
use crate::prepared::PreparedRequest;

fn named(name: &str) -> Value {
    json!({ "kind": "SCALAR", "name": name, "ofType": null })
}

fn object(name: &str) -> Value {
    json!({ "kind": "OBJECT", "name": name, "ofType": null })
}

fn non_null(inner: Value) -> Value {
    json!({ "kind": "NON_NULL", "name": null, "ofType": inner })
}

fn list(inner: Value) -> Value {
    json!({ "kind": "LIST", "name": null, "ofType": inner })
}

fn field(name: &str, args: Value, type_ref: Value) -> Value {
    json!({ "name": name, "description": null, "args": args, "type": type_ref })
}

fn argument(name: &str, type_ref: Value) -> Value {
    json!({ "name": name, "description": null, "type": type_ref })
}

fn introspection() -> Value {
    let user = object("User");
    json!({ "data": { "__schema": {
        "queryType": { "name": "Query" },
        "mutationType": { "name": "Mutation" },
        "subscriptionType": null,
        "types": [
            { "kind": "OBJECT", "name": "Query", "fields": [
                field("user", json!([argument("id", non_null(named("ID")))]), user.clone()),
                field("users", json!([]), non_null(list(non_null(user.clone())))),
                field("ping", json!([]), named("String")),
                field("search", json!([argument("text", named("String"))]),
                    json!({ "kind": "UNION", "name": "SearchResult", "ofType": null })),
            ]},
            { "kind": "OBJECT", "name": "Mutation", "fields": [
                field("createUser", json!([argument("input",
                    non_null(json!({ "kind": "INPUT_OBJECT", "name": "NewUser", "ofType": null })))]),
                    user.clone()),
            ]},
            { "kind": "OBJECT", "name": "User", "description": "A person", "fields": [
                field("id", json!([]), non_null(named("ID"))),
                field("name", json!([]), named("String")),
                field("friends", json!([argument("first", non_null(named("Int")))]), list(user)),
                field("role", json!([]), json!({ "kind": "ENUM", "name": "Role", "ofType": null })),
            ]},
            { "kind": "ENUM", "name": "Role", "enumValues": [{ "name": "ADMIN" }, { "name": "USER" }] },
            { "kind": "UNION", "name": "SearchResult" },
            { "kind": "INPUT_OBJECT", "name": "NewUser", "inputFields": [argument("name", named("String"))] },
            { "kind": "SCALAR", "name": "ID" },
            { "kind": "SCALAR", "name": "String" },
            { "kind": "SCALAR", "name": "Int" },
            { "kind": "OBJECT", "name": "__Type", "fields": [] },
        ]
    }}})
}

fn schema() -> GraphQlSchema {
    GraphQlSchema::from_response(introspection().to_string().as_bytes()).unwrap()
}

#[test]
fn root_fields_are_listed_with_their_signatures() {
    let schema = schema();
    let signatures: Vec<String> = schema
        .root_fields(OperationKind::Query)
        .iter()
        .map(|field| field.signature())
        .collect();
    assert_eq!(
        signatures,
        [
            "user(id: ID!): User",
            "users: [User!]!",
            "ping: String",
            "search(text: String): SearchResult"
        ]
    );
    assert_eq!(schema.root_fields(OperationKind::Mutation).len(), 1);
    assert!(schema.root_fields(OperationKind::Subscription).is_empty());
    assert_eq!(schema.type_count(), 9);
    assert_eq!(
        schema.type_named("Role").unwrap().enum_values,
        ["ADMIN", "USER"]
    );
    let required = TypeRef::NonNull(Box::new(TypeRef::Named("ID".to_string())));
    assert!(required.is_required());
    assert_eq!(required.named(), "ID");
}

#[test]
fn a_field_becomes_an_operation_selecting_its_simple_fields() {
    let operation = schema().operation(OperationKind::Query, "user").unwrap();
    assert_eq!(
        operation.query,
        "query User($id: ID!) {\n  user(id: $id) {\n    id\n    name\n    role\n  }\n}\n"
    );
    assert_eq!(Value::Object(operation.variables), json!({ "id": null }));
    let ping = schema().operation(OperationKind::Query, "ping").unwrap();
    assert_eq!(ping.query, "query Ping {\n  ping\n}\n");
    assert!(ping.variables.is_empty());
    let search = schema().operation(OperationKind::Query, "search").unwrap();
    assert!(search
        .query
        .contains("search(text: $text) {\n    __typename\n  }"));
    let created = schema()
        .operation(OperationKind::Mutation, "createUser")
        .unwrap();
    assert!(created
        .query
        .starts_with("mutation CreateUser($input: NewUser!) {"));
    assert!(schema()
        .operation(OperationKind::Query, "missing")
        .is_none());
}

#[test]
fn filtering_matches_names_and_types() {
    let schema = schema();
    let users = &schema.root_fields(OperationKind::Query)[1];
    assert!(users.matches(""));
    assert!(users.matches(" USER "));
    assert!(!users.matches("ping"));
}

#[test]
fn unusable_answers_explain_what_went_wrong() {
    let rejected = json!({ "errors": [{ "message": "introspection is disabled" }] });
    assert_eq!(
        GraphQlSchema::from_response(rejected.to_string().as_bytes()),
        Err(SchemaError::Rejected(
            "introspection is disabled".to_string()
        ))
    );
    assert_eq!(
        GraphQlSchema::from_response(b"<html>"),
        Err(SchemaError::NotJson)
    );
    let empty = json!({ "data": {} });
    assert_eq!(
        GraphQlSchema::from_response(empty.to_string().as_bytes()),
        Err(SchemaError::Missing)
    );
}

#[test]
fn the_schema_request_reuses_the_url_and_headers_with_a_json_body() {
    let request = PreparedRequest {
        method: HttpMethod::Get,
        url: "https://api.example.com/graphql".to_string(),
        headers: vec![
            Header::new("Authorization", "Bearer abc"),
            Header::new("content-type", "text/plain"),
        ],
        body: None,
        body_source: Default::default(),
    };
    let schema_request = request.for_schema();
    assert_eq!(schema_request.method, HttpMethod::Post);
    assert_eq!(schema_request.url, request.url);
    assert_eq!(
        schema_request.headers,
        vec![
            Header::new("Authorization", "Bearer abc"),
            Header::new("Content-Type", "application/json"),
        ]
    );
    let body: Value = serde_json::from_slice(&schema_request.body.unwrap()).unwrap();
    assert!(body["query"].as_str().unwrap().contains("__schema"));
}

#[test]
fn adding_an_operation_keeps_what_is_already_written() {
    let operation = schema().operation(OperationKind::Query, "user").unwrap();
    let mut body = BodyDraft::default();
    body.add_graphql_operation(&operation);
    assert_eq!(body.graphql_query, operation.query);
    body.graphql_variables = "{\"id\": \"7\", \"other\": 1}".to_string();
    body.add_graphql_operation(&operation);
    assert_eq!(body.graphql_query, format!("{0}\n{0}", operation.query));
    let variables: Value = serde_json::from_str(&body.graphql_variables).unwrap();
    assert_eq!(variables, json!({ "id": "7", "other": 1 }));
    body.graphql_variables = "not json".to_string();
    body.add_graphql_operation(&operation);
    assert_eq!(body.graphql_variables, "not json");
}
