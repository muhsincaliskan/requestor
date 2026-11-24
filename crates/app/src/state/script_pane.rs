use requestor_core::draft::ScriptDraft;
use requestor_core::script::{Phase, ScriptError, Unconverted};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScriptView {
    #[default]
    Script,
    JavaScript,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionFailure {
    Script(Vec<ScriptError>),
    JavaScript(Unconverted),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptPane {
    pub phase: Phase,
    pub view: ScriptView,
    pub failure: Option<ConversionFailure>,
}

impl Default for ScriptPane {
    fn default() -> Self {
        ScriptPane {
            phase: Phase::AfterResponse,
            view: ScriptView::default(),
            failure: None,
        }
    }
}

impl ScriptPane {
    pub fn select_phase(&mut self, phase: Phase) {
        if self.phase != phase {
            self.phase = phase;
            self.failure = None;
        }
    }

    pub fn select_view(&mut self, view: ScriptView) {
        if self.view != view {
            self.view = view;
            self.failure = None;
        }
    }

    pub fn convert_to_javascript(&mut self, draft: &mut ScriptDraft) {
        self.failure = draft
            .convert_to_javascript()
            .err()
            .map(ConversionFailure::Script);
        if self.failure.is_none() {
            self.view = ScriptView::JavaScript;
        }
    }

    pub fn convert_to_script(&mut self, draft: &mut ScriptDraft) -> bool {
        self.failure = draft
            .convert_to_script()
            .err()
            .map(ConversionFailure::JavaScript);
        let converted = self.failure.is_none();
        if converted {
            self.view = ScriptView::Script;
        }
        converted
    }
}
