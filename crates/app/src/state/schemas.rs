use requestor_core::graphql::GraphQlSchema;

use super::tab::TabId;
use super::{Action, AppState, Effect};

impl AppState {
    pub(super) fn dispatch_schema_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::FetchSchema(tab) => self.fetch_schema(tab, effects),
            Action::CancelSchema(tab) => self.cancel_schema(tab),
            Action::SchemaFetched { tab, job, result } => self.schema_fetched(tab, job, result),
            other => self.dispatch_history_action(other, effects),
        }
    }

    pub(super) fn cancel_schema(&mut self, id: TabId) {
        let job = self.tab_mut(id).and_then(|tab| tab.schema.cancel());
        self.abandoned_jobs.extend(job);
    }

    fn fetch_schema(&mut self, id: TabId, effects: &mut Vec<Effect>) {
        let Some(prepared) = self.prepare(id) else {
            return;
        };
        let request = match prepared {
            Ok(request) => request,
            Err(reason) => {
                if let Some(tab) = self.tab_mut(id) {
                    tab.schema.fail(reason);
                }
                return;
            }
        };
        self.warn_about_unresolved(&request);
        self.cancel_schema(id);
        self.next_job += 1;
        let job = self.next_job;
        if let Some(tab) = self.tab_mut(id) {
            tab.schema.start(job);
        }
        effects.push(Effect::FetchSchema {
            tab: id,
            job,
            request: Box::new(request.for_schema()),
            settings: Box::new(self.workspace.settings.request.clone()),
        });
    }

    fn schema_fetched(&mut self, id: TabId, job: u64, result: Result<GraphQlSchema, String>) {
        if let Some(tab) = self.tab_mut(id) {
            tab.schema.finish(job, result);
        }
    }
}
