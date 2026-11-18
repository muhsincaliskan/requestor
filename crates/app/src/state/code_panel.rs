use std::fmt;

use requestor_core::prepared::CodeLanguage;

use super::{Action, AppState, Effect, TabId};

const NO_REQUEST_TEXT: &str = "Open a request to see its code.";

#[derive(Clone, PartialEq)]
pub struct CodePanel {
    pub language: CodeLanguage,
    pub include_secrets: bool,
    pub code: Result<String, String>,
    tab: Option<TabId>,
}

impl fmt::Debug for CodePanel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CodePanel")
            .field("language", &self.language)
            .field("include_secrets", &self.include_secrets)
            .finish_non_exhaustive()
    }
}

impl CodePanel {
    fn new(language: CodeLanguage) -> Self {
        CodePanel {
            language,
            include_secrets: false,
            code: Ok(String::new()),
            tab: None,
        }
    }

    fn follow(&mut self, tab: Option<TabId>) {
        if self.tab != tab {
            self.tab = tab;
            self.include_secrets = false;
        }
    }

    pub fn with_options(&self, language: CodeLanguage, include_secrets: bool) -> Action {
        Action::CodePanelOptions {
            language,
            include_secrets,
        }
    }
}

impl AppState {
    pub(super) fn dispatch_code_panel_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::ToggleCodePanel => self.toggle_code_panel(),
            Action::CodePanelOptions {
                language,
                include_secrets,
            } => self.set_code_panel_options(language, include_secrets),
            other => self.dispatch_socket_action(other, effects),
        }
    }

    pub(super) fn refresh_code_panel(&mut self) {
        let active = self.active_tab;
        let Some(panel) = &mut self.code_panel else {
            return;
        };
        panel.follow(active);
        let (language, include_secrets) = (panel.language, panel.include_secrets);
        let code = match active {
            Some(tab) => self.code_for(tab, language, include_secrets),
            None => None,
        };
        let code = code.map_or_else(
            || Err(NO_REQUEST_TEXT.to_string()),
            |code| code.map_err(|error| error.to_string()),
        );
        if let Some(panel) = &mut self.code_panel {
            panel.code = code;
        }
    }

    fn toggle_code_panel(&mut self) {
        self.code_panel = match self.code_panel {
            Some(_) => None,
            None => Some(CodePanel::new(self.interface().code_language)),
        };
    }

    fn set_code_panel_options(&mut self, language: CodeLanguage, include_secrets: bool) {
        if let Some(panel) = &mut self.code_panel {
            panel.language = language;
            panel.include_secrets = include_secrets;
        }
        if self
            .workspace
            .settings
            .interface
            .remember_code_language(language)
        {
            self.unsaved.touch_settings(self.now);
        }
    }
}
