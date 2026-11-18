use serde_json::{json, Value};

use super::{FieldFilter, GraphQlSchema, OperationKind, SchemaError, TypeRef};
use crate::draft::BodyDraft;
use crate::header::Header;
use crate::method::HttpMethod;
use crate::prepared::PreparedRequest;
use crate::response::HttpResponse;

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
                field("node", json!(null), json!({ "kind": "INTERFACE", "name": "Node", "ofType": null })),
                field("ghost", json!([]), object("Ghost")),
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
            { "kind": "INTERFACE", "name": "Node", "fields": [
                field("id", json!([]), non_null(named("ID"))),
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
        .map(|field| field.signature().to_string())
        .collect();
    assert_eq!(
        signatures,
        [
            "user(id: ID!): User",
            "users: [User!]!",
            "ping: String",
            "search(text: String): SearchResult",
            "node: Node",
            "ghost: Ghost"
        ]
    );
    assert_eq!(schema.root_fields(OperationKind::Mutation).len(), 1);
    assert!(schema.root_fields(OperationKind::Subscription).is_empty());
    assert_eq!(schema.type_count(), 10);
    assert!(schema.type_named("Role").unwrap().is_leaf());
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
    assert!(users.matches(&FieldFilter::new("")));
    assert!(users.matches(&FieldFilter::new(" USER ")));
    assert!(!users.matches(&FieldFilter::new("ping")));
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
fn using_an_operation_replaces_the_query_and_keeps_filled_values() {
    let user = schema().operation(OperationKind::Query, "user").unwrap();
    let mut body = BodyDraft::default();
    body.graphql_query = "query Old { ping }".to_string();
    body.graphql_variables = "{\"id\": \"7\", \"other\": 1}".to_string();
    body.use_graphql_operation(user.clone());
    assert_eq!(body.graphql_query, user.query);
    let variables: Value = serde_json::from_str(&body.graphql_variables).unwrap();
    assert_eq!(variables, json!({ "id": "7" }));
    let ping = schema().operation(OperationKind::Query, "ping").unwrap();
    body.use_graphql_operation(ping);
    assert_eq!(body.graphql_variables, "");
    body.graphql_variables = "{\"id\": {{token}}}".to_string();
    body.use_graphql_operation(user);
    let variables: Value = serde_json::from_str(&body.graphql_variables).unwrap();
    assert_eq!(variables, json!({ "id": null }));
}

#[test]
fn errors_beside_a_schema_do_not_hide_it() {
    let mut answer = introspection();
    answer["errors"] = json!([{ "message": "deprecated field" }]);
    assert!(GraphQlSchema::from_response(answer.to_string().as_bytes()).is_ok());
    let nameless = json!({ "errors": [{}] });
    assert_eq!(
        GraphQlSchema::from_response(nameless.to_string().as_bytes()),
        Err(SchemaError::Rejected(String::new()))
    );
}

#[test]
fn a_type_nested_too_deep_is_reported() {
    let mut answer = introspection();
    answer["data"]["__schema"]["types"][0]["fields"][0]["args"][0]["type"] =
        json!({ "kind": "NON_NULL", "name": null, "ofType": null });
    assert_eq!(
        GraphQlSchema::from_response(answer.to_string().as_bytes()),
        Err(SchemaError::TooDeep)
    );
}

#[test]
fn a_cut_off_or_failed_answer_names_the_reason() {
    let mut response = HttpResponse {
        status: 200,
        reason: "OK".to_string(),
        version: "HTTP/1.1".to_string(),
        headers: Vec::new(),
        body: introspection().to_string().into_bytes(),
        truncated: true,
        elapsed: Default::default(),
        timing: None,
    };
    assert_eq!(response.graphql_schema(), Err(SchemaError::TooLarge));
    response.truncated = false;
    assert!(response.graphql_schema().is_ok());
    response.status = 502;
    response.body = b"Bad gateway".to_vec();
    assert_eq!(response.graphql_schema(), Err(SchemaError::Status(502)));
}
