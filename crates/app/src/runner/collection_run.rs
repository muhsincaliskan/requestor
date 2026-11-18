use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::Context;
use requestor_core::collection::RunSession;
use requestor_core::prepared::PreparedRequest;
use requestor_core::response::HttpResponse;
use requestor_core::transport::{Cancellation, HttpTransport, ReqwestTransport, TransportError};

use super::Runner;
use crate::state::{Action, CollectionRunJob};

const RUN_WORKER_NAME: &str = "requestor-collection-run";
const CANCEL_POLL: Duration = Duration::from_millis(50);

struct StoppableTransport<'a> {
    transport: ReqwestTransport,
    cancellation: &'a Cancellation,
}

struct RunReporter {
    job: u64,
    report: Sender<Action>,
    wake: Context,
}

impl Runner {
    pub fn run_collection(&mut self, run: CollectionRunJob, wake: &Context) {
        let job = run.job;
        let cancellation = Cancellation::default();
        self.running.retain(|_, running| !running.is_abandoned());
        self.running.insert(job, cancellation.clone());
        let reporter = RunReporter {
            job,
            report: self.report.clone(),
            wake: wake.clone(),
        };
        let spawned = thread::Builder::new()
            .name(RUN_WORKER_NAME.to_string())
            .spawn(move || {
                let error = execute(&run, &cancellation, &reporter).err();
                reporter.deliver(Action::RunFinished {
                    job: reporter.job,
                    error,
                });
            });
        if let Err(error) = spawned {
            let error = Some(error.to_string());
            if self.report.send(Action::RunFinished { job, error }).is_ok() {
                wake.request_repaint();
            }
        }
    }
}

fn execute(
    run: &CollectionRunJob,
    cancellation: &Cancellation,
    reporter: &RunReporter,
) -> Result<(), String> {
    let transport = StoppableTransport {
        transport: ReqwestTransport::new(&run.settings).map_err(|error| error.to_string())?,
        cancellation,
    };
    run_steps(run, cancellation, reporter, &transport);
    Ok(())
}

fn run_steps(
    run: &CollectionRunJob,
    cancellation: &Cancellation,
    reporter: &RunReporter,
    transport: &dyn HttpTransport,
) {
    let mut session = RunSession::new(
        run.collection_variables.clone(),
        run.environment.clone(),
        run.settings.clone(),
    );
    let mut completed = 0;
    for iteration in 1..=run.pacing.iterations() {
        for request in &run.requests {
            if cancellation.is_cancelled() {
                return;
            }
            let outcome = session.run_one(iteration, request, transport);
            if cancellation.is_cancelled() {
                return;
            }
            reporter.deliver(Action::RunStepFinished {
                job: run.job,
                outcome: Box::new(outcome),
            });
            completed += 1;
            if run.pacing.pauses_after(completed, run.requests.len()) {
                pause(run.pacing.delay(), cancellation);
            }
        }
    }
}

impl HttpTransport for StoppableTransport<'_> {
    fn send(&self, request: PreparedRequest) -> Result<HttpResponse, TransportError> {
        self.transport
            .send_cancellable(request, self.cancellation, &mut |_| {})
    }
}

fn pause(delay: Duration, cancellation: &Cancellation) {
    let until = Instant::now() + delay;
    while !cancellation.is_cancelled() {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return;
        }
        thread::sleep(left.min(CANCEL_POLL));
    }
}

impl RunReporter {
    fn deliver(&self, action: Action) {
        if self.report.send(action).is_ok() {
            self.wake.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests;
