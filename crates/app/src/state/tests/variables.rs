use requestor_core::collection::Variable;
use requestor_core::environment::Environment;
use requestor_core::workspace::{VariableAssignment, VariableHome};

use super::{state_with_request, LATER};
use crate::state::{Action, Effect};

fn assignment(home: VariableHome, value: &str) -> Action {
    Action::AssignVariable(VariableAssignment {
        home,
        name: "url".to_string(),
        value: value.to_string(),
    })
}

fn persisted_paths(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Persist(write) => Some(write.path().display().to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn an_inline_edit_updates_the_environment_and_saves_it() {
    let (mut state, _, _) = state_with_request();
    let id = state.workspace.add_environment(Environment::new("Staging"));
    let home = VariableHome::Environment {
        id: id.clone(),
        name: "Staging".to_string(),
    };
    state.apply(assignment(home, "https://new.test"), 0.0);
    let environment = state.workspace.environment(&id).unwrap();
    assert_eq!(
        environment.value_of("url").as_deref(),
        Some("https://new.test")
    );
    let saved = persisted_paths(&state.tick(LATER));
    assert!(
        saved.iter().any(|path| path.contains("environments")),
        "{saved:?}"
    );
}

#[test]
fn an_inline_edit_of_a_collection_variable_saves_the_collection() {
    let (mut state, collection_id, item_id) = state_with_request();
    state
        .workspace
        .collection_mut(&collection_id)
        .unwrap()
        .variable = Some(vec![Variable::text("url", "https://old.test")]);
    let home = VariableHome::Collection {
        id: collection_id.clone(),
        name: "Sample".to_string(),
    };
    state.apply(assignment(home, "https://new.test"), 0.0);
    let scope = state
        .workspace
        .scope_for(Some(&collection_id), Some(&item_id));
    assert_eq!(scope.resolve("{{url}}").text, "https://new.test");
    let saved = persisted_paths(&state.tick(LATER));
    assert!(
        saved.iter().any(|path| path.contains("collections")),
        "{saved:?}"
    );
}
