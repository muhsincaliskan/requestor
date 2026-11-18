use requestor_core::collection::{RunPacing, RunStepResult, RunnableRequest, Variable};
use requestor_core::environment::Environment;
use requestor_core::settings::RequestSettings;

use super::{Action, AppState, Effect, Overlay};

#[derive(Debug, Clone, PartialEq)]
pub struct RunnerStep {
    pub enabled: bool,
    request: RunnableRequest,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunnerForm {
    pub collection_id: String,
    pub collection_name: String,
    pub steps: Vec<RunnerStep>,
    pub iterations_text: String,
    pub delay_text: String,
    pub results: Vec<RunStepResult>,
    job: Option<u64>,
    expected: usize,
    confirming: bool,
    failure: Option<String>,
}

#[derive(Debug)]
pub struct CollectionRunJob {
    pub job: u64,
    pub requests: Vec<RunnableRequest>,
    pub collection_variables: Option<Vec<Variable>>,
    pub environment: Option<Environment>,
    pub settings: RequestSettings,
    pub pacing: RunPacing,
}

impl From<RunnableRequest> for RunnerStep {
    fn from(request: RunnableRequest) -> Self {
        RunnerStep {
            enabled: true,
            request,
        }
    }
}

impl RunnerStep {
    pub fn name(&self) -> String {
        self.request.qualified_name()
    }

    pub fn method(&self) -> &str {
        self.request.request.method_name()
    }
}

impl RunnerForm {
    fn for_collection(collection_id: &str, name: &str, runnable: Vec<RunnableRequest>) -> Self {
        let pacing = RunPacing::default();
        RunnerForm {
            collection_id: collection_id.to_string(),
            collection_name: name.to_string(),
            steps: runnable.into_iter().map(RunnerStep::from).collect(),
            iterations_text: pacing.iterations().to_string(),
            delay_text: pacing.delay_ms().to_string(),
            results: Vec::new(),
            job: None,
            expected: 0,
            confirming: false,
            failure: None,
        }
    }

    pub fn move_step(&mut self, index: usize, earlier: bool) {
        let target = if earlier {
            index.checked_sub(1)
        } else {
            Some(index + 1).filter(|next| *next < self.steps.len())
        };
        if let Some(target) = target.filter(|_| !self.is_running()) {
            self.steps.swap(index, target);
        }
    }

    pub fn pacing(&self) -> RunPacing {
        let default = RunPacing::default();
        let iterations = self.iterations_text.trim().parse();
        let delay = self.delay_text.trim().parse();
        RunPacing::new(
            iterations.unwrap_or(default.iterations()),
            delay.unwrap_or(default.delay_ms()),
        )
    }

    pub fn is_running(&self) -> bool {
        self.job.is_some()
    }

    pub fn can_start(&self) -> bool {
        !self.is_running() && self.steps.iter().any(|step| step.enabled)
    }

    pub fn is_confirming(&self) -> bool {
        self.confirming
    }

    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.results.len(), self.expected)
    }

    pub fn passed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.is_success())
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.results.len() - self.passed_count()
    }

    pub fn running_job(&self) -> Option<u64> {
        self.job
    }

    fn confirmed(&mut self, protected: bool) -> bool {
        let confirmed = !protected || self.confirming;
        self.confirming = !confirmed;
        confirmed
    }

    fn selected(&self) -> Vec<RunnableRequest> {
        self.steps
            .iter()
            .filter(|step| step.enabled)
            .map(|step| step.request.clone())
            .collect()
    }

    fn start(&mut self, job: u64, requests: usize) {
        self.job = Some(job);
        self.expected = self.pacing().step_count(requests);
        self.results.clear();
        self.failure = None;
    }

    fn record(&mut self, job: u64, result: RunStepResult) {
        if self.job == Some(job) {
            self.results.push(result);
        }
    }

    fn finish(&mut self, job: u64, error: Option<String>) {
        if self.job == Some(job) {
            self.job = None;
            self.failure = error;
        }
    }
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
            Action::StartRun => self.start_run(effects),
            Action::StopRun(job) => self.stop_run(job, effects),
            Action::RunStepFinished { job, result } => {
                if let Some(form) = self.runner_form_mut() {
                    form.record(job, *result);
                }
            }
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
        let protected = self.sends_to_protected_environment();
        let Some(form) = self.runner_form_mut() else {
            return;
        };
        if !form.can_start() || !form.confirmed(protected) {
            return;
        }
        let requests = form.selected();
        let pacing = form.pacing();
        self.next_job += 1;
        let job = self.next_job;
        let run = CollectionRunJob::new(job, requests, self, pacing);
        if let Some(form) = self.runner_form_mut() {
            form.start(job, run.requests.len());
        }
        effects.push(Effect::RunCollection(Box::new(run)));
    }

    fn stop_run(&mut self, job: u64, effects: &mut Vec<Effect>) {
        effects.push(Effect::AbortSend(job));
        if let Some(form) = self.runner_form_mut() {
            form.finish(job, None);
        }
    }
}
