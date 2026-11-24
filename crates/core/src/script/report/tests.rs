use super::{Applied, ScriptReport, VariableWrite, Written};
use crate::collection::Variable;
use crate::environment::Environment;
use crate::script::VariableTarget;

fn write(variable: &str, target: VariableTarget, value: Result<Written, &str>) -> VariableWrite {
    VariableWrite {
        variable: variable.to_string(),
        target,
        value: value.map_err(str::to_string),
    }
}

fn value(text: &str) -> Result<Written, &str> {
    Ok(Written::Value(text.to_string()))
}

#[test]
fn writes_go_to_their_target_and_failures_change_nothing() {
    let report = ScriptReport {
        writes: vec![
            write("token", VariableTarget::Environment, value("t-1")),
            write("id", VariableTarget::Collection, value("7")),
            write("lost", VariableTarget::Environment, Err("missing")),
            write("local", VariableTarget::Request, value("x")),
            write("old", VariableTarget::Environment, Ok(Written::Removed)),
        ],
        ..ScriptReport::default()
    };
    let mut environment = Environment::new("Dev");
    environment.set_value("token", "old");
    environment.set_value("old", "gone soon");
    let mut variables = Some(vec![Variable::text("id", "1")]);
    let applied = report.apply(Some(&mut environment), &mut variables);
    assert_eq!(
        applied,
        Applied {
            environment: true,
            collection: true
        }
    );
    assert_eq!(environment.value_of("token").as_deref(), Some("t-1"));
    assert_eq!(environment.value_of("old"), None);
    assert_eq!(environment.values.len(), 1);
    assert_eq!(variables.unwrap()[0].value_text(), "7");
}

#[test]
fn without_an_environment_values_land_in_the_collection() {
    let report = ScriptReport {
        writes: vec![write("token", VariableTarget::Environment, value("t"))],
        ..ScriptReport::default()
    };
    let mut variables = None;
    report.apply(None, &mut variables);
    assert_eq!(variables.map(|list| list.len()), Some(1));
    let mut untouched = None;
    ScriptReport::default().apply(None, &mut untouched);
    assert_eq!(untouched, None);
}
