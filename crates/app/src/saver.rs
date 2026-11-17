use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::state::Effect;

pub struct Saver {
    jobs: Option<Sender<Effect>>,
    failures: Receiver<String>,
    worker: Option<JoinHandle<()>>,
}

impl Saver {
    pub fn start() -> Self {
        let (jobs, queue) = channel::<Effect>();
        let (report, failures) = channel();
        let worker = thread::spawn(move || {
            for effect in queue {
                if let Err(reason) = effect.store() {
                    if report.send(reason).is_err() {
                        return;
                    }
                }
            }
        });
        Saver {
            jobs: Some(jobs),
            failures,
            worker: Some(worker),
        }
    }

    pub fn submit(&self, effect: Effect) -> Result<(), String> {
        match &self.jobs {
            Some(jobs) => jobs.send(effect).map_err(|error| error.to_string()),
            None => Ok(()),
        }
    }

    pub fn failures(&self) -> impl Iterator<Item = String> + '_ {
        self.failures.try_iter()
    }

    pub fn finish(&mut self) {
        self.jobs = None;
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                eprintln!("the save worker stopped unexpectedly");
            }
        }
    }
}
