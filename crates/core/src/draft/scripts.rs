use crate::collection::Event;
use crate::script::{
    Phase, RequestScripts, Script, ScriptError, Statement, StoredScript, Unconverted,
};

#[derive(Debug, Clone)]
pub struct ScriptDraft {
    phase: Phase,
    saved: StoredScript,
    saved_text: String,
    pub text: String,
    pub javascript: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptsDraft {
    pub before: ScriptDraft,
    pub after: ScriptDraft,
}

impl PartialEq for ScriptDraft {
    fn eq(&self, other: &Self) -> bool {
        self.phase == other.phase && self.saved == other.saved && self.text == other.text
    }
}

impl Default for ScriptsDraft {
    fn default() -> Self {
        ScriptsDraft {
            before: ScriptDraft::new(StoredScript::default(), Phase::BeforeRequest),
            after: ScriptDraft::new(StoredScript::default(), Phase::AfterResponse),
        }
    }
}

impl ScriptsDraft {
    pub fn from_events(events: Option<&Vec<Event>>) -> Self {
        let stored = RequestScripts::of_events(events);
        ScriptsDraft {
            before: ScriptDraft::new(stored.before, Phase::BeforeRequest),
            after: ScriptDraft::new(stored.after, Phase::AfterResponse),
        }
    }

    pub fn write_into(&self, events: &mut Option<Vec<Event>>) {
        self.before.write_into(events);
        self.after.write_into(events);
    }

    pub fn committed(&self) -> Self {
        ScriptsDraft {
            before: self.before.committed(),
            after: self.after.committed(),
        }
    }

    pub fn of_phase_mut(&mut self, phase: Phase) -> &mut ScriptDraft {
        match phase {
            Phase::BeforeRequest => &mut self.before,
            Phase::AfterResponse => &mut self.after,
        }
    }

    pub fn active_count(&self) -> usize {
        self.before.statement_count() + self.after.statement_count()
    }

    pub fn has_problems(&self) -> bool {
        self.before.has_problems() || self.after.has_problems()
    }
}

impl ScriptDraft {
    pub fn new(saved: StoredScript, phase: Phase) -> Self {
        let saved_text = match &saved {
            StoredScript::Converted(script) => script.text(),
            StoredScript::Unconverted { .. } => String::new(),
        };
        ScriptDraft {
            phase,
            javascript: saved.javascript().join("\n"),
            text: saved_text.clone(),
            saved_text,
            saved,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn is_edited(&self) -> bool {
        self.text != self.saved_text
    }

    pub fn parsed(&self) -> Result<Script, Vec<ScriptError>> {
        Script::parse(&self.text, self.phase)
    }

    pub fn runnable(&self) -> Option<Script> {
        if self.is_edited() {
            if let Ok(script) = self.parsed() {
                return (!script.is_empty()).then_some(script);
            }
        }
        self.saved.runnable().cloned()
    }

    pub fn unconverted(&self) -> Option<&Unconverted> {
        match &self.saved {
            StoredScript::Unconverted { problems, .. } if !self.is_edited() => Some(problems),
            _ => None,
        }
    }

    pub fn has_problems(&self) -> bool {
        self.unconverted().is_some() || self.parsed().is_err()
    }

    pub fn javascript_is_pending(&self) -> bool {
        self.javascript.trim() != self.current_javascript().join("\n").trim()
    }

    pub fn convert_to_javascript(&mut self) -> Result<(), Vec<ScriptError>> {
        let script = self.parsed()?;
        self.javascript = script.javascript_lines().join("\n");
        Ok(())
    }

    pub fn convert_to_script(&mut self) -> Result<(), Unconverted> {
        let lines: Vec<String> = self.javascript.lines().map(str::to_string).collect();
        let script = Script::from_javascript(&lines)?;
        self.text = script.text();
        if !self.is_edited() {
            self.javascript = self.current_javascript().join("\n");
        }
        Ok(())
    }

    pub fn session_text(&self) -> Option<String> {
        self.is_edited().then(|| self.text.clone())
    }

    fn current_javascript(&self) -> Vec<String> {
        match (self.is_edited(), self.parsed()) {
            (true, Ok(script)) => script.javascript_lines(),
            _ => self.saved.javascript(),
        }
    }

    fn write_into(&self, events: &mut Option<Vec<Event>>) {
        if !self.is_edited() {
            return;
        }
        if let Ok(script) = self.parsed() {
            script.write_into(events, self.phase);
        }
    }

    fn committed(&self) -> Self {
        match self.parsed() {
            Ok(script) if self.is_edited() => {
                let mut draft = ScriptDraft::new(StoredScript::Converted(script), self.phase);
                draft.text = self.text.clone();
                draft.saved_text = self.text.clone();
                draft
            }
            _ => self.clone(),
        }
    }

    fn statement_count(&self) -> usize {
        self.runnable().map_or(0, |script| {
            script
                .statements
                .iter()
                .filter(|statement| !matches!(statement, Statement::Blank | Statement::Comment(_)))
                .count()
        })
    }
}

#[cfg(test)]
mod tests;
