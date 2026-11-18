use std::thread;

use eframe::egui::Context;
use requestor_core::prepared::PreparedRequest;
use requestor_core::response::HttpResponse;
use requestor_core::settings::RequestSettings;
use requestor_core::transport::HttpTransport;

use super::Runner;
use crate::state::{Action, TabId};

const SCHEMA_WORKER_NAME: &str = "requestor-schema";

impl Runner {
    pub fn fetch_schema(
        &mut self,
        tab: TabId,
        job: u64,
        request: PreparedRequest,
        settings: &RequestSettings,
        wake: &Context,
    ) {
        let transport = match self.transport_for(settings) {
            Ok(transport) => transport,
            Err(reason) => return self.report_schema(tab, job, Err(reason)),
        };
        let report = self.report.clone();
        let wake = wake.clone();
        let spawned = thread::Builder::new()
            .name(SCHEMA_WORKER_NAME.to_string())
            .spawn(move || {
                let result = transport.send(request).map_err(|error| error.to_string());
                if report
                    .send(Action::SchemaFetched { tab, job, result })
                    .is_ok()
                {
                    wake.request_repaint();
                }
            });
        if let Err(error) = spawned {
            self.report_schema(tab, job, Err(error.to_string()));
        }
    }

    fn report_schema(&self, tab: TabId, job: u64, result: Result<HttpResponse, String>) {
        self.report
            .send(Action::SchemaFetched { tab, job, result })
            .ok();
    }
}
