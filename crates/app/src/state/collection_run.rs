mod form;

use requestor_core::checks::{RulesReport, VariableTarget};
use requestor_core::collection::{RunPacing, RunStepOutcome, RunnableRequest, Variable};
use requestor_core::environment::Environment;
use requestor_core::settings::RequestSettings;

use super::{Action, AppState, Effect, Overlay};
pub use form::RunnerForm;

#[derive(Debug)]
pub struct CollectionRunJob {
    pub job: u64,
    pub requests: Vec<RunnableRequest>,
    pub collection_variables: Option<Vec<Variable>>,
    pub environment: Option<Environment>,
    pub settings: RequestSettings,
    pub pacing: RunPacing,
}

impl CollectionRunJob {
    fn new(job: u64, requests: Vec<RunnableRequest>, state: &AppState, pacing: RunPacing) -> Self {
        let collection_variables = state
            .runner_collection()
            .and_then(|collection| collection.variable.clone());
        CollectionRunJob {
            job,
            requests,
            collection_variables,
            environment: state.workspace.active_environment().cloned(),
            settings: state.workspace.settings.request.clone(),
            pacing,
        }
    }
}

impl AppState {
    pub(super) fn dispatch_run_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::OpenRunner(collection_id) => self.open_runner(&collection_id),
            Action::CloseRunner => self.close_runner(),
            Action::StartRun => self.start_run(effects),
            Action::StopRun(job) => self.stop_run(job, effects),
            Action::RunStepFinished { job, outcome } => self.record_run_step(job, *outcome),
            Action::RunFinished { job, error } => {
                if let Some(form) = self.runner_form_mut() {
                    form.finish(job, error);
                }
            }
            other => self.dispatch_certificate_action(other, effects),
        }
    }

    fn runner_form_mut(&mut self) -> Option<&mut RunnerForm> {
        match self.overlay.as_mut()? {
            Overlay::CollectionRunner(form) => Some(form),
            _ => None,
        }
    }

    pub(super) fn running_collection_job(&self) -> Option<u64> {
        match self.overlay.as_ref()? {
            Overlay::CollectionRunner(form) => form.running_job(),
            _ => None,
        }
    }

    fn close_runner(&mut self) {
        if matches!(self.overlay, Some(Overlay::CollectionRunner(_))) {
            self.overlay = None;
        }
    }

    fn runner_collection(&self) -> Option<&requestor_core::collection::Collection> {
        match self.overlay.as_ref()? {
            Overlay::CollectionRunner(form) => self.workspace.collection(&form.collection_id),
            _ => None,
        }
    }

    fn open_runner(&mut self, collection_id: &str) {
        let Some(collection) = self.workspace.collection(collection_id) else {
            return;
        };
        let runnable = collection.runnable_requests();
        let form = RunnerForm::for_collection(collection_id, &collection.info.name, runnable);
        self.overlay = Some(Overlay::CollectionRunner(form));
    }

    fn start_run(&mut self, effects: &mut Vec<Effect>) {
        let protected_id = self
            .workspace
            .active_environment()
            .filter(|environment| environment.is_protected())
            .and_then(|environment| environment.id.clone());
        let Some(form) = self.runner_form_mut() else {
            return;
        };
        if !form.can_start() || !form.confirm(protected_id.as_deref()) {
            return;
        }
        let requests = form.selected();
        let pacing = form.pacing();
        self.next_job += 1;
        let job = self.next_job;
        let run = CollectionRunJob::new(job, requests, self, pacing);
        if let Some(form) = self.runner_form_mut() {
            let environment_id = run.environment.as_ref().and_then(|env| env.id.clone());
            form.start(job, run.requests.len(), environment_id);
        }
        effects.push(Effect::RunCollection(Box::new(run)));
    }

    fn record_run_step(&mut self, job: u64, outcome: RunStepOutcome) {
        let Some(form) = self
            .runner_form_mut()
            .filter(|form| form.running_job() == Some(job))
        else {
            return;
        };
        form.record(outcome.result);
        if outcome.extracted.is_empty() {
            return;
        }
        let collection_id = form.collection_id.clone();
        let environment_id = form.environment_id().map(str::to_string);
        let mut report = RulesReport {
            extracted: outcome.extracted,
            ..RulesReport::default()
        };
        if environment_id
            .as_deref()
            .is_some_and(|id| self.workspace.environment(id).is_none())
        {
            report
                .extracted
                .retain(|entry| entry.extraction.target == VariableTarget::Collection);
        }
        let changed = self.workspace.store_extracted_in(
            &report,
            Some(&collection_id),
            environment_id.as_deref(),
        );
        self.mark_extraction_changes(changed);
    }

    fn stop_run(&mut self, job: u64, effects: &mut Vec<Effect>) {
        effects.push(Effect::AbortSend(job));
        if let Some(form) = self.runner_form_mut() {
            form.finish(job, None);
        }
    }
}
