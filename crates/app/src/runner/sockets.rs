use std::collections::HashMap;
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender};
use std::thread;

use eframe::egui::Context;
use requestor_core::prepared::PreparedRequest;
use requestor_core::settings::RequestSettings;
use requestor_core::transport::{SocketCommand, SocketTransport, TungsteniteTransport};

use crate::state::{Action, TabId};

const SOCKET_WORKER_NAME: &str = "requestor-socket";
const QUEUED_EVENTS: usize = 256;

struct SocketWorker {
    commands: Option<Sender<SocketCommand>>,
    events: Receiver<Action>,
}

#[derive(Default)]
pub struct SocketWorkers {
    workers: HashMap<u64, SocketWorker>,
}

impl SocketWorkers {
    pub fn open(
        &mut self,
        tab: TabId,
        job: u64,
        request: PreparedRequest,
        settings: &RequestSettings,
        wake: &Context,
    ) -> Result<(), String> {
        let (commands, received) = channel();
        let (report, events) = sync_channel(QUEUED_EVENTS);
        let transport = TungsteniteTransport::new(settings);
        let wake = wake.clone();
        thread::Builder::new()
            .name(SOCKET_WORKER_NAME.to_string())
            .spawn(move || {
                let event_report = report.clone();
                let event_wake = wake.clone();
                let result = transport
                    .open(&request, &received, &mut |event| {
                        let action = Action::SocketEvent { tab, job, event };
                        event_wake.request_repaint();
                        event_report.send(action).ok();
                    })
                    .map_err(|error| error.to_string());
                report.send(Action::SocketEnded { tab, job, result }).ok();
                wake.request_repaint();
            })
            .map_err(|error| error.to_string())?;
        let worker = SocketWorker {
            commands: Some(commands),
            events,
        };
        self.workers.insert(job, worker);
        Ok(())
    }

    pub fn send(&mut self, job: u64, text: String) {
        if let Some(commands) = self
            .workers
            .get(&job)
            .and_then(|worker| worker.commands.as_ref())
        {
            commands.send(SocketCommand::Send(text)).ok();
        }
    }

    pub fn stop(&mut self, job: u64) {
        if let Some(worker) = self.workers.get_mut(&job) {
            worker.commands = None;
        }
    }

    pub fn drain(&mut self) -> Vec<Action> {
        let mut drained = Vec::new();
        let mut ended = Vec::new();
        for (job, worker) in &self.workers {
            for action in worker.events.try_iter() {
                if matches!(action, Action::SocketEnded { .. }) {
                    ended.push(*job);
                }
                drained.push(action);
            }
        }
        for job in ended {
            self.workers.remove(&job);
        }
        drained
    }
}
