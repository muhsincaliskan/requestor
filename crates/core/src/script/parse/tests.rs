use serde_json::json;

use crate::script::{
    Comparison, Condition, Dynamic, JsonPath, Operand, Phase, Script, SetValue, Statement, Subject,
    VariableTarget,
};

fn after(text: &str) -> Script {
    Script::parse(text, Phase::AfterResponse).unwrap()
}

fn errors(text: &str, phase: Phase) -> Vec<(usize, String)> {
    Script::parse(text, phase)
        .unwrap_err()
        .into_iter()
        .map(|error| (error.line, error.message))
        .collect()
}

#[test]
fn commands_parse_into_statements() {
    let script = after(
        "expect status is 200\nsave json data.token as token in collection\nset id to new uuid in request\nclear old\nfail \"stop\"",
    );
    assert_eq!(
        script.statements,
        vec![
            Statement::Expect {
                condition: Condition::Compare {
                    subject: Subject::Status,
                    comparison: Comparison::Is,
                    operand: Some(Operand::Literal(json!(200))),
                },
                name: None,
            },
            Statement::Save {
                subject: Subject::Json(JsonPath::parse("data.token").unwrap()),
                variable: "token".into(),
                target: VariableTarget::Collection,
            },
            Statement::Set {
                variable: "id".into(),
                value: SetValue::Dynamic(Dynamic::Uuid),
                target: VariableTarget::Request,
            },
            Statement::Clear {
                variable: "old".into(),
                target: VariableTarget::Environment,
            },
            Statement::Fail("stop".into()),
        ]
    );
}

#[test]
fn conditions_bind_not_then_and_then_or() {
    let script = after("expect not {{a}} exists and status is 1 or body is empty named \"x\"");
    let Statement::Expect { condition, name } = &script.statements[0] else {
        panic!("expected an expectation");
    };
    assert_eq!(name.as_deref(), Some("x"));
    let Condition::Any(parts) = condition else {
        panic!("or at the top");
    };
    assert!(matches!(&parts[0], Condition::All(inner) if matches!(inner[0], Condition::Not(_))));
    assert_eq!(
        parts[1],
        Condition::Compare {
            subject: Subject::Body,
            comparison: Comparison::IsEmpty,
            operand: None,
        }
    );
}

#[test]
fn if_blocks_nest_with_else_if_and_else() {
    let script = after(
        "if status is 200\n  save json token as token\nelse if status is 401\n  if {{x}} is 1\n  end\nelse\n  fail \"no\"\nend",
    );
    let Statement::If(conditional) = &script.statements[0] else {
        panic!("expected if");
    };
    assert_eq!(conditional.branches.len(), 2);
    assert!(matches!(conditional.branches[1].body[0], Statement::If(_)));
    assert_eq!(conditional.otherwise.as_ref().unwrap().len(), 1);
}

#[test]
fn printing_gives_back_the_same_text() {
    let text = "# login\nexpect time below 500 ms\nexpect json is not empty\n\nif status is 200 and header Content-Type contains \"json\"\n  save json data.items[0].id as id in collection\nelse\n  set token to \"\"\n  set n to random number 1 10\nend\nexpect {{env}} is \"prod\" named \"Env\"";
    assert_eq!(after(text).text(), text);
}

#[test]
fn mistakes_report_their_line_and_a_hint() {
    assert_eq!(
        errors(
            "expect status is 200\nsav json a as b",
            Phase::AfterResponse
        ),
        vec![(
            2,
            "unknown command \"sav\" (did you mean \"save\"?)".to_string()
        )]
    );
    assert_eq!(
        errors("if status is 1\nexpect body is empty", Phase::AfterResponse)[0].0,
        1
    );
    assert_eq!(errors("end", Phase::AfterResponse)[0].0, 1);
    assert!(errors("set x to", Phase::AfterResponse)[0]
        .1
        .contains("expected a value"));
    assert!(errors("save body as b in nowhere", Phase::AfterResponse)[0]
        .1
        .contains("nowhere"));
}

#[test]
fn response_values_are_refused_before_the_request() {
    let found = errors("expect status is 200", Phase::BeforeRequest);
    assert!(found[0].1.contains("After response"));
    assert!(Script::parse(
        "if {{env}} is \"prod\"\n  set x to 1\nend",
        Phase::BeforeRequest
    )
    .is_ok());
}

#[test]
fn blank_tail_lines_are_dropped() {
    assert_eq!(after("set a to 1\n\n\n").statements.len(), 1);
    assert!(after("").is_empty());
}
