use requestor_core::checks::RulesReport;
use requestor_core::collection::Request;
use requestor_core::prepared::PreparedRequest;
use requestor_core::response::HttpResponse;
use requestor_core::secret::Redactor;
use requestor_core::variables::VariableScope;

use super::exchange::Exchange;
use super::tab::TabId;
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

    fn sendable(&self, id: TabId) -> Option<(Request, VariableScope)> {
        let tab = self.tab(id)?;
        let collection_id = tab
            .binding
            .as_ref()
            .map(|bound| bound.collection_id.as_str());
        let item_id = tab.binding.as_ref().map(|bound| bound.item_id.as_str());
        let request =
            self.workspace
                .sendable_request(collection_id, item_id, tab.draft.to_request());
        Some((request, self.workspace.scope_for(collection_id, item_id)))
    }

    fn prepare(&self, id: TabId) -> Option<Result<PreparedRequest, String>> {
        let (request, scope) = self.sendable(id)?;
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
            self.warn(format!(
                "Not defined: {}. Add them to the environment or collection.",
                unresolved.join(", ")
            ));
        }
        self.next_job += 1;
        let job = self.next_job;
        let replaced = self
            .tab_mut(id)
            .and_then(|tab| tab.exchange.start_send(job));
        self.abandoned_jobs.extend(replaced);
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
        let Some(tab) = self.tab(id).filter(|tab| tab.is_waiting_for(job)) else {
            return;
        };
        let rules = tab.draft.rules.to_rules();
        let collection_id = tab
            .binding
            .as_ref()
            .map(|bound| bound.collection_id.clone());
        let previous = self
            .tab_mut(id)
            .and_then(|tab| tab.exchange.take_previous());
        let exchange = match result {
            Ok(response) => {
                let report = (!rules.is_empty()).then(|| rules.evaluate(&response));
                if let Some(report) = &report {
                    self.store_extracted(report, collection_id.as_deref());
                }
                let view = ResponseView::from_response(response, detect_format)
                    .with_report(report)
                    .with_previous(previous);
                Exchange::Completed(Box::new(view))
            }
            Err(reason) => Exchange::Failed(reason),
        };
        if let Some(tab) = self.tab_mut(id) {
            tab.exchange = exchange;
        }
    }

    fn store_extracted(&mut self, report: &RulesReport, collection_id: Option<&str>) {
        let changed = self.workspace.store_extracted(report, collection_id);
        let now = self.now;
        if let Some(environment_id) = changed.environment_id {
            self.unsaved.touch_environment(&environment_id, now);
        }
        if let Some(collection_id) = changed.collection_id {
            self.unsaved.touch_collection(&collection_id, now);
        }
    }

    fn copy_as_curl(&mut self, id: TabId, include_secrets: bool, effects: &mut Vec<Effect>) {
        let Some((request, scope)) = self.sendable(id) else {
            return;
        };
        let settings = &self.workspace.settings.request;
        match PreparedRequest::from_request(&request, &scope, settings) {
            Ok(prepared) if include_secrets => {
                effects.push(Effect::CopyText(prepared.to_curl_with_secrets()));
            }
            Ok(prepared) => {
                let redactor = Redactor::for_request(&scope, request.auth.as_ref());
                effects.push(Effect::CopyText(prepared.to_curl(&redactor)));
            }
            Err(error) => self.report_error(error.to_string()),
        }
    }
}
