use serde_json::Map;

use super::ResponseRules;
use crate::collection::{Event, Script};
use crate::text::StringOrList;

const TEST_LISTEN: &str = "test";
const SCRIPT_KIND: &str = "text/javascript";

impl ResponseRules {
    pub fn of_events(events: Option<&Vec<Event>>) -> Option<ResponseRules> {
        let tests: Vec<&Event> = events
            .into_iter()
            .flatten()
            .filter(|event| event.listen == TEST_LISTEN)
            .collect();
        match tests.as_slice() {
            [] => Some(ResponseRules::default()),
            [event] if event.is_enabled() => {
                let exec = event
                    .script
                    .as_ref()
                    .and_then(|script| script.exec.as_ref());
                ResponseRules::from_script(&exec.map(StringOrList::items).unwrap_or_default())
            }
            _ => None,
        }
    }

    pub fn write_into(&self, events: &mut Option<Vec<Event>>) {
        let list = events.get_or_insert_with(Vec::new);
        if self.is_empty() {
            list.retain(|event| event.listen != TEST_LISTEN);
        } else {
            let exec = Some(StringOrList::List(self.script_lines()));
            match list.iter_mut().find(|event| event.listen == TEST_LISTEN) {
                Some(event) => event.script.get_or_insert_with(test_script).exec = exec,
                None => list.push(test_event(exec)),
            }
        }
        if list.is_empty() {
            *events = None;
        }
    }
}

fn test_script() -> Script {
    Script {
        id: None,
        kind: Some(SCRIPT_KIND.to_string()),
        exec: None,
        src: None,
        name: None,
        extra: Map::new(),
    }
}

fn test_event(exec: Option<StringOrList>) -> Event {
    Event {
        id: None,
        listen: TEST_LISTEN.to_string(),
        script: Some(Script {
            exec,
            ..test_script()
        }),
        disabled: None,
        extra: Map::new(),
    }
}

#[cfg(test)]
mod tests {
    use crate::checks::{Check, ResponseRules};
    use crate::collection::Event;

    fn events(json: &str) -> Option<Vec<Event>> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn rules_are_written_into_a_test_event_and_read_back() {
        let rules = ResponseRules {
            checks: vec![Check::Status(200)],
            extractions: Vec::new(),
        };
        let mut stored = events(r#"[{"listen":"prerequest","script":{"exec":["x"]}}]"#);
        rules.write_into(&mut stored);
        assert_eq!(stored.as_ref().unwrap().len(), 2);
        assert_eq!(ResponseRules::of_events(stored.as_ref()), Some(rules));
        ResponseRules::default().write_into(&mut stored);
        assert_eq!(stored.as_ref().unwrap().len(), 1);
        let mut only_tests = events(r#"[{"listen":"test","id":"keep","script":{"exec":[]}}]"#);
        ResponseRules::default().write_into(&mut only_tests);
        assert_eq!(only_tests, None);
    }

    #[test]
    fn hand_written_or_disabled_test_scripts_are_left_to_postman() {
        let custom = events(r#"[{"listen":"test","script":{"exec":["console.log(1);"]}}]"#);
        assert_eq!(ResponseRules::of_events(custom.as_ref()), None);
        let disabled = events(r#"[{"listen":"test","disabled":true,"script":{"exec":[]}}]"#);
        assert_eq!(ResponseRules::of_events(disabled.as_ref()), None);
        assert_eq!(
            ResponseRules::of_events(None),
            Some(ResponseRules::default())
        );
    }
}
