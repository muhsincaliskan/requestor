use requestor_core::collection::{Request, BEFORE_REQUEST_STOPPED};
use requestor_core::environment::Environment;
use requestor_core::prepared::PreparedRequest;
use requestor_core::response::HttpResponse;
use requestor_core::settings::RequestSettings;
use requestor_core::variables::VariableScope;

use super::exchange::Exchange;
use super::scripting::with_locals;
use super::tab::{RequestTab, TabId};
use super::{Action, AppState, Effect, Overlay};
use crate::response::ResponseView;

impl AppState {
    pub(super) fn dispatch_sending_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::Send(tab) => self.send_guarded(tab, effects),
            Action::SendConfirmed(tab) => self.send(tab, effects),
            Action::CancelSend(tab) => self.cancel_send(tab),
            Action::StreamProgress { tab, job, update } => self.stream_progress(tab, job, update),
            Action::SendFinished { tab, job, result } => self.finish_send(tab, job, result),
            Action::CopyAsCurl {
                tab,
                include_secrets,
            } => self.copy_as_curl(tab, include_secrets, effects),
            Action::OpenCode(tab) => self.open_code(tab),
            Action::ShowCode {
                tab,
                language,
                include_secrets,
            } => self.show_code(tab, language, include_secrets),
            other => self.dispatch_code_panel_action(other, effects),
        }
    }

    pub(super) fn sendable(&self, id: TabId) -> Option<(Request, VariableScope)> {
        let tab = self.tab(id)?;
        let collection_id = tab.scope().map(|scope| scope.collection_id.as_str());
        let item_id = tab.scope().map(|scope| scope.item_id.as_str());
        let request =
            self.workspace
                .sendable_request(collection_id, item_id, tab.draft.to_request());
        Some((request, self.workspace.scope_for(collection_id, item_id)))
    }

    pub(super) fn request_settings(&self, id: TabId) -> RequestSettings {
        let global = &self.workspace.settings.request;
        self.tab(id).map_or_else(
            || global.clone(),
            |tab| tab.draft.overrides.apply_to(global),
        )
    }

    pub(super) fn prepare(&self, id: TabId) -> Option<Result<PreparedRequest, String>> {
        self.prepare_with(id, &[])
    }

    fn prepare_with(
        &self,
        id: TabId,
        locals: &[(String, String)],
    ) -> Option<Result<PreparedRequest, String>> {
        let (request, scope) = self.sendable(id)?;
        let scope = with_locals(scope, locals);
        Some(
            PreparedRequest::from_request(&request, &scope, &self.request_settings(id))
                .map_err(|error| error.to_string()),
        )
    }

    pub(super) fn warn_about_unresolved(&mut self, request: &PreparedRequest) {
        let unresolved = request.unresolved_variables();
        if !unresolved.is_empty() {
            self.warn(format!(
                "Not defined: {}. Add them to the environment or collection.",
                unresolved.join(", ")
            ));
        }
    }

    pub fn sends_to_protected_environment(&self) -> bool {
        self.workspace
            .active_environment()
            .is_some_and(Environment::is_protected)
    }

    fn send_guarded(&mut self, id: TabId, effects: &mut Vec<Effect>) {
        let starts_exchange = self.tab(id).is_some_and(|tab| !tab.is_connected());
        if starts_exchange && self.sends_to_protected_environment() {
            if self.overlay.is_none() {
                self.overlay = Some(Overlay::ConfirmProtectedSend(id));
            }
            return;
        }
        self.send(id, effects);
    }

    fn send(&mut self, id: TabId, effects: &mut Vec<Effect>) {
        if self.tab(id).is_some_and(RequestTab::is_connected) {
            return self.send_socket_message(id, effects);
        }
        if let Some(tab) = self.tab_mut(id) {
            tab.remember_body_presentation();
        }
        let before = self.run_before_script(id);
        if let Some(failure) = &before.failure {
            return self.fail_send(id, format!("{BEFORE_REQUEST_STOPPED}{failure}"));
        }
        let Some(prepared) = self.prepare_with(id, &before.request_variables()) else {
            return;
        };
        let request = match prepared {
            Ok(request) => request,
            Err(reason) => return self.fail_send(id, reason),
        };
        self.warn_about_unresolved(&request);
        if request.is_websocket() {
            return self.connect(id, request, effects);
        }
        self.next_job += 1;
        let job = self.next_job;
        let replaced = self.tab_mut(id).and_then(|tab| {
            let record = tab.history_entry();
            tab.exchange.start_send(job, record)
        });
        self.abandoned_jobs.extend(replaced);
        if let Some(tab) = self.tab_mut(id) {
            tab.before_report = Some(before);
        }
        effects.push(Effect::Send {
            tab: id,
            job,
            request: Box::new(request),
            settings: Box::new(self.request_settings(id)),
        });
    }

    fn fail_send(&mut self, id: TabId, reason: String) {
        let replaced = self.tab_mut(id).and_then(|tab| {
            tab.before_report = None;
            tab.exchange.fail(reason)
        });
        self.abandoned_jobs.extend(replaced);
    }

    pub(super) fn send_finished(
        &mut self,
        id: TabId,
        job: u64,
        result: Result<HttpResponse, String>,
    ) {
        let detect_format = self.interface().response_format_detection;
        let Some(tab) = self.tab(id).filter(|tab| tab.is_waiting_for(job)) else {
            return;
        };
        let after = tab.draft.scripts.after.runnable();
        let collection_id = tab
            .binding
            .as_ref()
            .map(|bound| bound.collection_id.clone());
        let previous = self
            .tab_mut(id)
            .and_then(|tab| tab.exchange.take_previous());
        let record = self.tab_mut(id).and_then(|tab| tab.exchange.take_record());
        let before = self.tab_mut(id).and_then(|tab| tab.before_report.take());
        let status = result.as_ref().ok().map(|response| response.status);
        self.remember_sent(record.map(|record| record.finished(status)));
        let exchange = match result {
            Ok(response) => {
                let report = self.run_after_script(
                    id,
                    after.as_ref(),
                    before,
                    &response,
                    collection_id.as_deref(),
                );
                let presentation = self
                    .tab_mut(id)
                    .and_then(|tab| tab.body_presentation.take());
                let view = ResponseView::from_response(response, detect_format)
                    .with_report(report)
                    .with_previous(previous)
                    .with_presentation(presentation);
                Exchange::Completed(Box::new(view))
            }
            Err(reason) => Exchange::Failed(reason),
        };
        if let Some(tab) = self.tab_mut(id) {
            tab.exchange = exchange;
        }
    }
}
