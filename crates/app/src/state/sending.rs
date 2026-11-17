use requestor_core::prepared::PreparedRequest;
use requestor_core::response::HttpResponse;

use super::tab::{Exchange, TabId};
use super::{Action, AppState, Effect};
use crate::response::ResponseView;

impl AppState {
    pub(super) fn dispatch_sending_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::Send(tab) => self.send(tab, effects),
            Action::CancelSend(tab) => self.cancel_send(tab),
            Action::SendFinished { tab, job, result } => self.send_finished(tab, job, result),
            Action::CopyAsCurl {
                tab,
                include_secrets,
            } => self.copy_as_curl(tab, include_secrets, effects),
            other => self.dispatch_library_action(other, effects),
        }
    }

    fn prepare(&self, id: TabId) -> Option<Result<PreparedRequest, String>> {
        let tab = self.tab(id)?;
        let collection_id = tab
            .binding
            .as_ref()
            .map(|bound| bound.collection_id.as_str());
        let item_id = tab.binding.as_ref().map(|bound| bound.item_id.as_str());
        let request =
            self.workspace
                .sendable_request(collection_id, item_id, tab.draft.to_request());
        let scope = self.workspace.scope_for(collection_id, item_id);
        Some(
            PreparedRequest::from_request(&request, &scope, &self.workspace.settings.request)
                .map_err(|error| error.to_string()),
        )
    }

    fn send(&mut self, id: TabId, effects: &mut Vec<Effect>) {
        let Some(prepared) = self.prepare(id) else {
            return;
        };
        let request = match prepared {
            Ok(request) => request,
            Err(reason) => {
                if let Some(tab) = self.tab_mut(id) {
                    tab.exchange = Exchange::Failed(reason);
                }
                return;
            }
        };
        let unresolved = request.unresolved_variables();
        if !unresolved.is_empty() {
            self.warn(format!("Unresolved variables: {}", unresolved.join(", ")));
        }
        self.abandon_send(id);
        self.next_job += 1;
        let job = self.next_job;
        if let Some(tab) = self.tab_mut(id) {
            tab.exchange = Exchange::Sending { job };
        }
        effects.push(Effect::Send {
            tab: id,
            job,
            request: Box::new(request),
            settings: Box::new(self.workspace.settings.request.clone()),
        });
    }

    fn cancel_send(&mut self, id: TabId) {
        self.abandon_send(id);
    }

    fn send_finished(&mut self, id: TabId, job: u64, result: Result<HttpResponse, String>) {
        let detect_format = self.interface().response_format_detection;
        let Some(tab) = self.tab_mut(id).filter(|tab| tab.is_waiting_for(job)) else {
            return;
        };
        tab.exchange = match result {
            Ok(response) => Exchange::Completed(Box::new(ResponseView::from_response(
                response,
                detect_format,
            ))),
            Err(reason) => Exchange::Failed(reason),
        };
    }

    fn copy_as_curl(&mut self, id: TabId, include_secrets: bool, effects: &mut Vec<Effect>) {
        match self.prepare(id) {
            Some(Ok(request)) if include_secrets => {
                effects.push(Effect::CopyText(request.to_curl_with_secrets()));
            }
            Some(Ok(request)) => effects.push(Effect::CopyText(request.to_curl())),
            Some(Err(reason)) => self.report_error(reason),
            None => {}
        }
    }
}
