use requestor_core::collection::{RunPacing, RunStepResult, RunnableRequest};

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
    results: Vec<RunStepResult>,
    passed: usize,
    environment_id: Option<String>,
    job: Option<u64>,
    expected: usize,
    confirming: Option<String>,
    failure: Option<String>,
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
    pub(super) fn for_collection(
        collection_id: &str,
        name: &str,
        runnable: Vec<RunnableRequest>,
    ) -> Self {
        let pacing = RunPacing::default();
        RunnerForm {
            collection_id: collection_id.to_string(),
            collection_name: name.to_string(),
            steps: runnable.into_iter().map(RunnerStep::from).collect(),
            iterations_text: pacing.iterations().to_string(),
            delay_text: pacing.delay_ms().to_string(),
            results: Vec::new(),
            passed: 0,
            environment_id: None,
            job: None,
            expected: 0,
            confirming: None,
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
        self.confirming.is_some()
    }

    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.results.len(), self.expected)
    }

    pub fn results(&self) -> &[RunStepResult] {
        &self.results
    }

    pub fn passed_count(&self) -> usize {
        self.passed
    }

    pub fn failed_count(&self) -> usize {
        self.results.len() - self.passed_count()
    }

    pub fn running_job(&self) -> Option<u64> {
        self.job
    }

    pub(super) fn confirm(&mut self, protected_id: Option<&str>) -> bool {
        if protected_id.is_none() || self.confirming.as_deref() == protected_id {
            self.confirming = None;
            return true;
        }
        self.confirming = protected_id.map(str::to_string);
        false
    }

    pub(super) fn environment_id(&self) -> Option<&str> {
        self.environment_id.as_deref()
    }

    pub(super) fn selected(&self) -> Vec<RunnableRequest> {
        self.steps
            .iter()
            .filter(|step| step.enabled)
            .map(|step| step.request.clone())
            .collect()
    }

    pub(super) fn start(&mut self, job: u64, requests: usize, environment_id: Option<String>) {
        self.job = Some(job);
        self.environment_id = environment_id;
        self.expected = self.pacing().step_count(requests);
        self.results.clear();
        self.passed = 0;
        self.failure = None;
    }

    pub(super) fn record(&mut self, result: RunStepResult) {
        self.passed += usize::from(result.is_success());
        self.results.push(result);
    }

    pub(super) fn finish(&mut self, job: u64, error: Option<String>) {
        if self.job == Some(job) {
            self.job = None;
            self.failure = error;
        }
    }
}
