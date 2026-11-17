use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

use eframe::egui::Context;
use requestor_core::prepared::PreparedRequest;
use requestor_core::settings::RequestSettings;
use requestor_core::transport::{Cancellation, ReqwestTransport};

use crate::state::{Action, TabId};

const WORKER_NAME: &str = "requestor-send";

struct SharedTransport {
    settings: RequestSettings,
    transport: Arc<ReqwestTransport>,
}

pub struct Runner {
    finished: Receiver<Action>,
    report: Sender<Action>,
    shared: Option<SharedTransport>,
    running: HashMap<u64, Cancellation>,
}

impl Runner {
    pub fn new() -> Self {
        let (report, finished) = channel();
        Runner {
            finished,
            report,
            shared: None,
            running: HashMap::new(),
        }
    }

    pub fn send(
        &mut self,
        tab: TabId,
        job: u64,
        request: PreparedRequest,
        settings: &RequestSettings,
        wake: &Context,
    ) {
        let transport = match self.transport_for(settings) {
            Ok(transport) => transport,
            Err(reason) => return self.report_failure(tab, job, reason),
        };
        let report = self.report.clone();
        let wake = wake.clone();
        let cancellation = Cancellation::default();
        self.running.retain(|_, running| !running.is_abandoned());
        self.running.insert(job, cancellation.clone());
        let spawned = thread::Builder::new()
            .name(WORKER_NAME.to_string())
            .spawn(move || {
                let result = transport
                    .send_cancellable(request, &cancellation)
                    .map_err(|error| error.to_string());
                let delivered = report.send(Action::SendFinished { tab, job, result });
                if delivered.is_ok() {
                    wake.request_repaint();
                }
            });
        if let Err(error) = spawned {
            self.report_failure(tab, job, error.to_string());
        }
    }

    pub fn abort(&mut self, job: u64) {
        if let Some(cancellation) = self.running.remove(&job) {
            cancellation.cancel();
        }
    }

    pub fn finished(&self) -> impl Iterator<Item = Action> + '_ {
        self.finished.try_iter()
    }

    fn transport_for(
        &mut self,
        settings: &RequestSettings,
    ) -> Result<Arc<ReqwestTransport>, String> {
        if let Some(shared) = self
            .shared
            .as_ref()
            .filter(|shared| shared.settings == *settings)
        {
            return Ok(Arc::clone(&shared.transport));
        }
        let transport =
            Arc::new(ReqwestTransport::new(settings).map_err(|error| error.to_string())?);
        self.shared = Some(SharedTransport {
            settings: settings.clone(),
            transport: Arc::clone(&transport),
        });
        Ok(transport)
    }

    fn report_failure(&self, tab: TabId, job: u64, reason: String) {
        let result = Err(reason);
        if self
            .report
            .send(Action::SendFinished { tab, job, result })
            .is_err()
        {
            unreachable!("the runner owns the receiving end");
        }
    }
}
