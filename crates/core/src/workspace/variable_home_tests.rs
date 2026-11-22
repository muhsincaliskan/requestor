use super::{VariableAssignment, VariableHome, Workspace};
use crate::collection::{Collection, Item, Request, Url, Variable};
use crate::environment::Environment;

struct Sample {
    workspace: Workspace,
    collection_id: String,
    folder_id: String,
    request_id: String,
}

fn sample() -> Sample {
    let mut collection = Collection::new("Sample");
    collection.variable = Some(vec![Variable::text("host", "collection.test")]);
    let folder_id = collection
        .add_item(None, Item::folder("Users".to_string(), Vec::new()))
        .unwrap();
    collection.find_item_mut(&folder_id).unwrap().variable =
        Some(vec![Variable::text("shared", "folder")]);
    let request = Request::new("GET", Url::Raw("https://{{host}}/{{shared}}".to_string()));
    let request_id = collection
        .add_item(
            Some(&folder_id),
            Item::for_request("list".to_string(), request),
        )
        .unwrap();
    let mut workspace = Workspace::default();
    let collection_id = workspace.add_collection(collection);
    Sample {
        workspace,
        collection_id,
        folder_id,
        request_id,
    }
}

fn assign(workspace: &mut Workspace, home: &VariableHome, name: &str, value: &str) -> bool {
    workspace.assign_variable(&VariableAssignment {
        home: home.clone(),
        name: name.to_string(),
        value: value.to_string(),
    })
}

fn activate(workspace: &mut Workspace, values: &str) -> String {
    let environment: Environment =
        serde_json::from_str(&format!(r#"{{"name":"Staging","values":{values}}}"#)).unwrap();
    let id = workspace.add_environment(environment);
    workspace.session.active_environment = Some(id.clone());
    id
}

#[test]
fn a_variable_is_edited_where_its_winning_value_lives() {
    let mut sample = sample();
    let environment_id = activate(&mut sample.workspace, r#"[{"key":"shared","value":"env"}]"#);
    let homes = sample
        .workspace
        .variable_homes(Some(&sample.collection_id), Some(&sample.request_id));
    assert!(matches!(
        homes.home_of("shared"),
        Some(VariableHome::Environment { id, .. }) if *id == environment_id
    ));
    assert!(matches!(
        homes.home_of("host"),
        Some(VariableHome::Collection { name, .. }) if name == "Sample"
    ));
    assert!(matches!(
        homes.home_of("missing"),
        Some(VariableHome::Environment { .. })
    ));
}

#[test]
fn folder_variables_win_over_the_collection_without_an_environment() {
    let mut sample = sample();
    sample
        .workspace
        .collection_mut(&sample.collection_id)
        .unwrap()
        .variable = Some(vec![Variable::text("shared", "collection")]);
    let homes = sample
        .workspace
        .variable_homes(Some(&sample.collection_id), Some(&sample.request_id));
    let home = homes.home_of("shared").unwrap().clone();
    assert!(matches!(
        &home,
        VariableHome::Folder { folder_id, name, .. } if *folder_id == sample.folder_id && name == "Users"
    ));
    assert!(assign(&mut sample.workspace, &home, "shared", "edited"));
    let scope = sample
        .workspace
        .scope_for(Some(&sample.collection_id), Some(&sample.request_id));
    assert_eq!(scope.resolve("{{shared}}").text, "edited");
    assert!(matches!(
        homes.home_of("missing"),
        Some(VariableHome::Collection { .. })
    ));
}

#[test]
fn setting_a_variable_updates_or_adds_it_in_its_home() {
    let mut sample = sample();
    let id = activate(&mut sample.workspace, r#"[{"key":"token","value":"old"}]"#);
    let home = VariableHome::Environment {
        id: id.clone(),
        name: "Staging".to_string(),
    };
    assert!(assign(&mut sample.workspace, &home, "token", "new"));
    assert!(assign(&mut sample.workspace, &home, "added", "value"));
    let environment = sample.workspace.environment(&id).unwrap();
    assert_eq!(environment.value_of("token").as_deref(), Some("new"));
    assert_eq!(environment.value_of("added").as_deref(), Some("value"));
    let gone = VariableHome::Environment {
        id: "gone".to_string(),
        name: String::new(),
    };
    assert!(!assign(&mut sample.workspace, &gone, "token", "x"));
}

#[test]
fn a_request_outside_collections_without_an_environment_has_no_home() {
    let sample = sample();
    assert_eq!(
        sample.workspace.variable_homes(None, None).home_of("x"),
        None
    );
}

#[test]
fn disabled_or_valueless_entries_do_not_claim_a_variable() {
    let mut sample = sample();
    let id = activate(
        &mut sample.workspace,
        r#"[{"key":"host"},{"key":"url","value":"old","enabled":false}]"#,
    );
    let homes = sample
        .workspace
        .variable_homes(Some(&sample.collection_id), Some(&sample.request_id));
    assert!(matches!(
        homes.home_of("host"),
        Some(VariableHome::Collection { .. })
    ));
    let home = homes.home_of("url").unwrap().clone();
    assert!(assign(&mut sample.workspace, &home, "url", "new"));
    let environment = sample.workspace.environment(&id).unwrap();
    let urls: Vec<_> = environment
        .values
        .iter()
        .filter(|entry| entry.key == "url")
        .map(|entry| (entry.value_text(), entry.is_enabled()))
        .collect();
    assert_eq!(
        urls,
        [("old".to_string(), false), ("new".to_string(), true)]
    );
}

#[test]
fn the_winning_duplicate_is_the_one_edited() {
    let mut sample = sample();
    let id = activate(
        &mut sample.workspace,
        r#"[{"key":"url","value":"first"},{"key":"url","value":"winner"}]"#,
    );
    let home = VariableHome::Environment {
        id,
        name: "Staging".to_string(),
    };
    assert!(assign(&mut sample.workspace, &home, "url", "edited"));
    let scope = sample
        .workspace
        .scope_for(Some(&sample.collection_id), Some(&sample.request_id));
    assert_eq!(scope.resolve("{{url}}").text, "edited");
}
