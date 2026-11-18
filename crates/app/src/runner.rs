use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

use eframe::egui::Context;
use requestor_core::prepared::PreparedRequest;
use requestor_core::settings::RequestSettings;
use requestor_core::transport::{
    Cancellation, ReqwestTransport, SocketCommand, SocketTransport, TungsteniteTransport,
};
use requestor_core::update::{newer_release, GithubReleaseClient, Version, LATEST_RELEASE_URL};

use crate::state::{Action, StreamUpdate, TabId};

const WORKER_NAME: &str = "requestor-send";
const UPDATE_WORKER_NAME: &str = "requestor-update-check";
const SOCKET_WORKER_NAME: &str = "requestor-socket";

struct SharedTransport {
    settings: RequestSettings,
    transport: Arc<ReqwestTransport>,
}

pub struct Runner {
    finished: Receiver<Action>,
    report: Sender<Action>,
    shared: Option<SharedTransport>,
    running: HashMap<u64, Cancellation>,
    sockets: HashMap<u64, Sender<SocketCommand>>,
}

impl Runner {
    pub fn new() -> Self {
        let (report, finished) = channel();
        Runner {
            finished,
            report,
            shared: None,
            running: HashMap::new(),
            sockets: HashMap::new(),
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
                let progress_report = report.clone();
                let progress_wake = wake.clone();
                let result = transport
                    .send_cancellable(request, &cancellation, &mut |progress| {
                        let update = StreamUpdate::from(progress);
                        let action = Action::StreamProgress { tab, job, update };
                        if progress_report.send(action).is_ok() {
                            progress_wake.request_repaint();
                        }
                    })
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

    pub fn open_socket(
        &mut self,
        tab: TabId,
        job: u64,
        request: PreparedRequest,
        settings: &RequestSettings,
        wake: &Context,
    ) {
        let (commands, received) = channel();
        let transport = TungsteniteTransport::new(settings);
        let report = self.report.clone();
        let wake = wake.clone();
        let spawned = thread::Builder::new()
            .name(SOCKET_WORKER_NAME.to_string())
            .spawn(move || {
                let event_report = report.clone();
                let event_wake = wake.clone();
                let result = transport
                    .open(&request, &received, &mut |event| {
                        let action = Action::SocketEvent { tab, job, event };
                        if event_report.send(action).is_ok() {
                            event_wake.request_repaint();
                        }
                    })
                    .map_err(|error| error.to_string());
                if report
                    .send(Action::SocketEnded { tab, job, result })
                    .is_ok()
                {
                    wake.request_repaint();
                }
            });
        match spawned {
            Ok(_) => {
                self.sockets.insert(job, commands);
            }
            Err(error) => {
                let result = Err(error.to_string());
                self.report
                    .send(Action::SocketEnded { tab, job, result })
                    .ok();
            }
        }
    }

    pub fn socket_send(&mut self, job: u64, text: String) {
        let delivered = self
            .sockets
            .get(&job)
            .is_some_and(|commands| commands.send(SocketCommand::Send(text)).is_ok());
        if !delivered {
            self.sockets.remove(&job);
        }
    }

    pub fn check_for_update(&self, settings: &RequestSettings, wake: &Context) {
        let settings = settings.for_update_check();
        let report = self.report.clone();
        let wake = wake.clone();
        thread::Builder::new()
            .name(UPDATE_WORKER_NAME.to_string())
            .spawn(move || {
                let Ok(transport) = ReqwestTransport::new(&settings) else {
                    return;
                };
                let client = GithubReleaseClient::new(transport, LATEST_RELEASE_URL);
                if let Ok(Some(release)) = newer_release(&client, &Version::current()) {
                    if report.send(Action::UpdateAvailable(release)).is_ok() {
                        wake.request_repaint();
                    }
                }
            })
            .ok();
    }

    pub fn abort(&mut self, job: u64) {
        if let Some(cancellation) = self.running.remove(&job) {
            cancellation.cancel();
        }
        self.sockets.remove(&job);
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
