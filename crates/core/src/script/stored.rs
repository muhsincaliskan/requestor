use serde_json::Map;

use super::{Phase, Script, Unconverted, UnconvertedLine};
use crate::collection::{Event, Script as EventScript};
use crate::text::StringOrList;

const PRE_REQUEST_LISTEN: &str = "prerequest";
const TEST_LISTEN: &str = "test";
const SCRIPT_KIND: &str = "text/javascript";
const TURNED_OFF: &str = "this script is turned off in Postman";

#[derive(Debug, Clone, PartialEq)]
pub enum StoredScript {
    Converted(Script),
    Unconverted {
        javascript: Vec<String>,
        problems: Unconverted,
    },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RequestScripts {
    pub before: StoredScript,
    pub after: StoredScript,
}

impl Default for StoredScript {
    fn default() -> Self {
        StoredScript::Converted(Script::default())
    }
}

impl Phase {
    pub fn listen(self) -> &'static str {
        match self {
            Phase::BeforeRequest => PRE_REQUEST_LISTEN,
            Phase::AfterResponse => TEST_LISTEN,
        }
    }
}

impl RequestScripts {
    pub fn of_events(events: Option<&Vec<Event>>) -> Self {
        RequestScripts {
            before: StoredScript::of_events(events, Phase::BeforeRequest),
            after: StoredScript::of_events(events, Phase::AfterResponse),
        }
    }
}

impl StoredScript {
    pub fn of_events(events: Option<&Vec<Event>>, phase: Phase) -> Self {
        let Some(event) = events
            .into_iter()
            .flatten()
            .find(|event| event.listen == phase.listen())
        else {
            return StoredScript::default();
        };
        let javascript = event
            .script
            .as_ref()
            .and_then(|script| script.exec.as_ref())
            .map(StringOrList::items)
            .unwrap_or_default()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !event.is_enabled() {
            return StoredScript::turned_off(javascript);
        }
        match Script::from_javascript(&javascript) {
            Ok(script) => StoredScript::Converted(script),
            Err(problems) => StoredScript::Unconverted {
                javascript,
                problems,
            },
        }
    }

    fn turned_off(javascript: Vec<String>) -> Self {
        StoredScript::Unconverted {
            javascript,
            problems: Unconverted {
                lines: vec![UnconvertedLine {
                    line: 1,
                    text: String::new(),
                    reason: TURNED_OFF.to_string(),
                }],
            },
        }
    }

    pub fn runnable(&self) -> Option<&Script> {
        match self {
            StoredScript::Converted(script) if !script.is_empty() => Some(script),
            _ => None,
        }
    }

    pub fn javascript(&self) -> Vec<String> {
        match self {
            StoredScript::Converted(script) => script.javascript_lines(),
            StoredScript::Unconverted { javascript, .. } => javascript.clone(),
        }
    }
}

impl Script {
    pub fn write_into(&self, events: &mut Option<Vec<Event>>, phase: Phase) {
        let listen = phase.listen();
        let list = events.get_or_insert_with(Vec::new);
        if self.is_empty() {
            list.retain(|event| event.listen != listen);
        } else {
            let exec = Some(StringOrList::List(self.javascript_lines()));
            match list.iter_mut().find(|event| event.listen == listen) {
                Some(event) => {
                    event.script.get_or_insert_with(empty_script).exec = exec;
                    event.disabled = None;
                }
                None => list.push(new_event(listen, exec)),
            }
        }
        if list.is_empty() {
            *events = None;
        }
    }
}

fn empty_script() -> EventScript {
    EventScript {
        id: None,
        kind: Some(SCRIPT_KIND.to_string()),
        exec: None,
        src: None,
        name: None,
        extra: Map::new(),
    }
}

fn new_event(listen: &str, exec: Option<StringOrList>) -> Event {
    Event {
        id: None,
        listen: listen.to_string(),
        script: Some(EventScript {
            exec,
            ..empty_script()
        }),
        disabled: None,
        extra: Map::new(),
    }
}
