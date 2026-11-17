use serde_json::json;

use crate::checks::{Check, ExtractSource, Extraction, JsonPath, ResponseRules, VariableTarget};

fn sample() -> ResponseRules {
    ResponseRules {
        checks: vec![
            Check::Status(200),
            Check::ResponseTimeBelow(500),
            Check::BodyContains("say \"ok\"".to_string()),
            Check::HeaderContains {
                name: "Content-Type".to_string(),
                text: "json".to_string(),
            },
            Check::JsonEquals {
                path: JsonPath::parse("data.items[0].id").unwrap(),
                expected: json!({"a": [1, "x"]}),
            },
        ],
        extractions: vec![
            Extraction {
                variable: "token".to_string(),
                source: ExtractSource::Json(JsonPath::parse("data.content-type").unwrap()),
                target: VariableTarget::Environment,
            },
            Extraction {
                variable: "session".to_string(),
                source: ExtractSource::Header("X-Session".to_string()),
                target: VariableTarget::Collection,
            },
        ],
    }
}

#[test]
fn rules_become_postman_lines_and_come_back_unchanged() {
    let rules = sample();
    let lines = rules.script_lines();
    assert_eq!(
        lines[0],
        r#"pm.test("Status is 200", () => pm.response.to.have.status(200));"#
    );
    assert_eq!(
        lines[4],
        r#"pm.test("JSON data.items[0].id equals {\"a\":[1,\"x\"]}", () => pm.expect(pm.response.json().data.items[0].id).to.eql({"a":[1,"x"]}));"#
    );
    assert_eq!(
        lines[5],
        r#"pm.environment.set("token", pm.response.json().data["content-type"]);"#
    );
    assert_eq!(
        lines[6],
        r#"pm.collectionVariables.set("session", pm.response.headers.get("X-Session"));"#
    );
    let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_eq!(ResponseRules::from_script(&borrowed), Some(rules));
}

#[test]
fn hand_written_scripts_are_not_taken_as_rules() {
    for line in [
        "pm.test(\"Status is 200\", function () { pm.response.to.have.status(200); });",
        "pm.test(\"Status is 201\", () => pm.response.to.have.status(200));",
        "console.log(pm.response.text());",
    ] {
        assert_eq!(ResponseRules::from_script(&[line]), None, "{line}");
    }
    assert_eq!(
        ResponseRules::from_script(&["", "  "]),
        Some(ResponseRules::default())
    );
}
