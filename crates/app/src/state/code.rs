use requestor_core::error::CoreError;
use requestor_core::prepared::{CodeLanguage, PreparedRequest};
use requestor_core::secret::Redactor;

use super::overlay::{CodeForm, Overlay};
use super::tab::TabId;
use super::{AppState, Effect};

impl AppState {
    pub(super) fn copy_as_curl(
        &mut self,
        id: TabId,
        include_secrets: bool,
        effects: &mut Vec<Effect>,
    ) {
        match self.code_for(id, CodeLanguage::Curl, include_secrets) {
            Some(Ok(code)) => effects.push(Effect::CopyText(code)),
            Some(Err(error)) => self.report_error(error.to_string()),
            None => {}
        }
    }

    pub(super) fn open_code(&mut self, id: TabId) {
        self.show_code(id, self.interface().code_language, false);
    }

    pub(super) fn show_code(&mut self, id: TabId, language: CodeLanguage, include_secrets: bool) {
        match self.code_for(id, language, include_secrets) {
            Some(Ok(code)) => {
                self.overlay = Some(Overlay::Code(CodeForm {
                    tab: id,
                    language,
                    include_secrets,
                    code,
                }));
                if self
                    .workspace
                    .settings
                    .interface
                    .remember_code_language(language)
                {
                    self.unsaved.touch_settings(self.now);
                }
            }
            Some(Err(error)) => self.report_error(error.to_string()),
            None => {}
        }
    }

    pub(super) fn code_for(
        &self,
        id: TabId,
        language: CodeLanguage,
        include_secrets: bool,
    ) -> Option<Result<String, CoreError>> {
        let (request, scope) = self.sendable(id)?;
        let settings = self.request_settings(id);
        let prepared = match PreparedRequest::referencing_files(&request, &scope, &settings) {
            Ok(prepared) => prepared,
            Err(error) => return Some(Err(error)),
        };
        Some(Ok(if include_secrets {
            prepared.to_code_with_secrets(language)
        } else {
            let redactor = Redactor::for_request(&scope, request.auth.as_ref());
            prepared.to_code(language, &redactor)
        }))
    }
}
