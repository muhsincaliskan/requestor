use super::Workspace;
use crate::collection::{Auth, Collection, Item, Request, Url, Variable};
use crate::environment::Environment;

fn environment(name: &str, json_values: &str) -> Environment {
    serde_json::from_str(&format!(r#"{{"name":"{name}","values":{json_values}}}"#)).unwrap()
}

fn workspace_with_request() -> (Workspace, String, String) {
    let mut collection = Collection::new("Sample");
    collection.variable = Some(vec![
        Variable::text("host", "collection.test"),
        Variable::text("shared", "collection"),
    ]);
    let folder = collection
        .add_item(None, Item::folder("Users".to_string(), Vec::new()))
        .unwrap();
    collection.find_item_mut(&folder).unwrap().variable =
        Some(vec![Variable::text("shared", "folder")]);
    let request = Request::new("GET", Url::Raw("https://{{host}}/{{shared}}".to_string()));
    let item = collection
        .add_item(
            Some(&folder),
            Item::for_request("list".to_string(), request),
        )
        .unwrap();
    let mut workspace = Workspace::default();
    let collection_id = workspace.add_collection(collection);
    (workspace, collection_id, item)
}

#[test]
fn adding_a_collection_twice_gives_the_copy_its_own_identity() {
    let mut workspace = Workspace::default();
    let mut collection = Collection::new("Sample");
    collection.ensure_ids();
    let first = workspace.add_collection(collection.clone());
    let second = workspace.add_collection(collection);
    assert_ne!(first, second);
    assert!(workspace.collection(&first).is_some());
    assert!(workspace.remove_collection(&first).is_some());
    assert!(workspace.collection(&first).is_none());
    assert!(workspace.collection(&second).is_some());
}

#[test]
fn the_environment_overrides_folder_variables_which_override_the_collection() {
    let (mut workspace, collection_id, item) = workspace_with_request();
    let scope = workspace.scope_for(Some(&collection_id), Some(&item));
    assert_eq!(
        scope.resolve("{{host}}/{{shared}}").text,
        "collection.test/folder"
    );
    let id = workspace.add_environment(environment(
        "Staging",
        r#"[{"key":"shared","value":"environment"},{"key":"off","value":"x","enabled":false}]"#,
    ));
    workspace.session.active_environment = Some(id.clone());
    let scope = workspace.scope_for(Some(&collection_id), Some(&item));
    assert_eq!(scope.resolve("{{shared}}").text, "environment");
    assert_eq!(
        workspace.variable_names(Some(&collection_id), Some(&item)),
        ["host", "shared"]
    );
    workspace.remove_environment(&id);
    assert!(workspace.active_environment().is_none());
    assert_eq!(
        workspace
            .scope_for(None, None)
            .resolve("{{shared}}")
            .unresolved,
        ["shared"]
    );
}

#[test]
fn a_request_without_auth_is_sent_with_the_inherited_auth() {
    let (mut workspace, collection_id, item) = workspace_with_request();
    workspace.collection_mut(&collection_id).unwrap().auth = Some(Auth::bearer("inherited"));
    let stored = workspace
        .collection(&collection_id)
        .unwrap()
        .find_item(&item)
        .unwrap()
        .full_request()
        .unwrap();
    let inherited = workspace.sendable_request(Some(&collection_id), Some(&item), stored.clone());
    assert_eq!(inherited.auth.unwrap().value_of("token"), Some("inherited"));
    let mut explicit = stored.clone();
    explicit.auth = Some(Auth::none());
    let kept = workspace.sendable_request(Some(&collection_id), Some(&item), explicit);
    assert!(kept.auth.unwrap().is_none());
    assert!(workspace
        .sendable_request(None, None, stored)
        .auth
        .is_none());
}

#[test]
fn a_workspace_has_requests_once_it_holds_a_collection() {
    let mut workspace = Workspace::default();
    workspace.add_environment(Environment::new("Staging"));
    assert!(!workspace.has_requests());
    workspace.add_collection(Collection::new("Shop"));
    assert!(workspace.has_requests());
}
