use requestor_core::response::HttpResponse;

use super::tab::TabId;
use super::{Action, AppState, Effect};

impl AppState {
    pub(super) fn dispatch_schema_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::FetchSchema(tab) => self.fetch_schema(tab, effects),
            Action::SchemaFetched { tab, job, result } => self.schema_fetched(tab, job, result),
            other => self.dispatch_history_action(other, effects),
        }
    }

    fn fetch_schema(&mut self, id: TabId, effects: &mut Vec<Effect>) {
        let Some(prepared) = self.prepare(id) else {
            return;
        };
        let request = match prepared {
            Ok(request) => request.for_schema(),
            Err(reason) => {
                if let Some(tab) = self.tab_mut(id) {
                    tab.schema.fail(reason);
                }
                return;
            }
        };
        self.next_job += 1;
        let job = self.next_job;
        if let Some(tab) = self.tab_mut(id) {
            tab.schema.start(job);
        }
        effects.push(Effect::FetchSchema {
            tab: id,
            job,
            request: Box::new(request),
            settings: Box::new(self.workspace.settings.request.clone()),
        });
    }

    fn schema_fetched(&mut self, id: TabId, job: u64, result: Result<HttpResponse, String>) {
        let schema = result
            .and_then(|response| response.graphql_schema().map_err(|error| error.to_string()));
        if let Some(tab) = self.tab_mut(id) {
            tab.schema.finish(job, schema);
        }
    }
}
