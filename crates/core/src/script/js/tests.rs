use crate::script::{Phase, Script};

fn lines(source: &str) -> Vec<String> {
    source.lines().map(str::to_string).collect()
}

fn converted(source: &str) -> String {
    Script::from_javascript(&lines(source)).unwrap().text()
}

fn round_trip(text: &str) {
    let script = Script::parse(text, Phase::AfterResponse).unwrap();
    let javascript = script.javascript_lines();
    let back = Script::from_javascript(&javascript)
        .unwrap_or_else(|failure| panic!("{failure:?}\n{javascript:?}"));
    assert_eq!(back, script, "{javascript:#?}");
}

#[test]
fn scripts_survive_a_trip_through_javascript() {
    round_trip("expect status is 200\nexpect time below 500 ms\nexpect body contains \"ok\"\nexpect header Content-Type contains \"json\"\nexpect json data.id is 5");
    round_trip("expect status is not 404 named \"Not missing\"\nexpect json data.items is not empty\nexpect header X-Id exists\nexpect header X-Old not exists\nexpect {{env}} exists\nexpect json data.x not exists");
    round_trip("# keep\nsave json data.token as token\nsave header X-Id as id in collection\nsave status as last in request\nclear token\nset n to -3\nset t to timestamp\nset ms to timestamp ms\nset r to random number 1 100\nset iso to now iso\nset id to new uuid\nset s to \"Bearer {{token}}\"\nset b to true");
    round_trip("if status is 200 and json ok is true or not body contains \"x\"\n  set a to 1\n\n  set b to 2\nelse if {{env}} is \"prod\"\n  fail \"stop\"\nelse\n  expect json a above 1\nend");
    round_trip("expect not status is 200\nexpect status is 200 and json list contains {{id}}\nexpect body is empty");
}

#[test]
fn requestor_generated_lines_are_kept_byte_for_byte() {
    let classic = [
        "pm.test(\"Status is 200\", () => pm.response.to.have.status(200));",
        "pm.test(\"Response time is below 500 ms\", () => pm.expect(pm.response.responseTime).to.be.below(500));",
        "pm.test(\"Body contains \\\"ok\\\"\", () => pm.expect(pm.response.text()).to.include(\"ok\"));",
        "pm.test(\"Header Content-Type contains \\\"json\\\"\", () => pm.expect(pm.response.headers.get(\"Content-Type\")).to.include(\"json\"));",
        "pm.test(\"JSON data.id equals 5\", () => pm.expect(pm.response.json().data.id).to.eql(5));",
        "pm.environment.set(\"token\", pm.response.json().data.token);",
        "pm.collectionVariables.set(\"id\", pm.response.headers.get(\"X-Id\"));",
    ]
    .map(str::to_string)
    .to_vec();
    let script = Script::from_javascript(&classic).unwrap();
    assert_eq!(script.javascript_lines(), classic);
    assert_eq!(
        script.text(),
        "expect status is 200\nexpect time below 500 ms\nexpect body contains \"ok\"\nexpect header Content-Type contains \"json\"\nexpect json data.id is 5\nsave json data.token as token\nsave header X-Id as id in collection"
    );
}

#[test]
fn common_postman_styles_are_recognized() {
    let source = r#"var jsonData = JSON.parse(responseBody);
const data = pm.response.json().data;
pm.test("Status code is 200", function () {
    pm.response.to.have.status(200);
});
pm.test('Has token', function () {
    pm.expect(jsonData.token).to.not.be.undefined;
    pm.expect(data.items.length).to.be.above(0);
});
postman.setEnvironmentVariable("token", jsonData.token);
tests["Body ok"] = responseCode.code === 200;
if (pm.response.code === 401) {
    pm.environment.unset('token');
}"#;
    let failure = Script::from_javascript(&lines(source)).unwrap_err();
    assert_eq!(failure.lines.len(), 1);
    assert_eq!(failure.lines[0].line, 6);
    let supported = source.replace(
        "    pm.expect(data.items.length).to.be.above(0);\n",
        "    pm.expect(data.items).to.not.be.empty;\n",
    );
    assert_eq!(
        converted(&supported),
        "expect status is 200 named \"Status code is 200\"\nexpect json token exists and json data.items is not empty named \"Has token\"\nsave json token as token\nexpect status is 200 named \"Body ok\"\nif status is 401\n  clear token\nend"
    );
}

#[test]
fn unknown_javascript_is_reported_by_line_and_kept_out() {
    let failure = Script::from_javascript(&lines(
        "pm.test(\"ok\", () => pm.response.to.have.status(200));\nconst sig = CryptoJS.HmacSHA256(a, b);\nitems.forEach(i => console.log(i));",
    ))
    .unwrap_err();
    let found: Vec<(usize, &str)> = failure
        .lines
        .iter()
        .map(|line| (line.line, line.text.as_str()))
        .collect();
    assert_eq!(
        found,
        vec![
            (2, "const sig = CryptoJS.HmacSHA256(a, b);"),
            (3, "items.forEach(i => console.log(i));"),
        ]
    );
    assert!(Script::from_javascript(&lines("pm.test(\"x\", () => {")).is_err());
    assert!(Script::from_javascript(&lines("pm.globals.set(\"a\", 1);")).is_err());
}

#[test]
fn empty_javascript_is_an_empty_script() {
    assert!(Script::from_javascript(&[]).unwrap().is_empty());
    assert_eq!(converted("// note\n\n// more"), "# note\n\n# more");
}

#[test]
fn status_compared_with_text_stays_unconverted() {
    for source in [
        "pm.test(\"ok\", () => pm.response.to.have.status(\"OK\"));",
        "if (pm.response.code === \"200\") {\n}",
        "pm.expect(pm.response.code).to.eql(\"200\");",
    ] {
        assert!(Script::from_javascript(&lines(source)).is_err(), "{source}");
    }
}
