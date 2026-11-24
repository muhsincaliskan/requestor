use std::fmt;
use std::time::Duration;

use super::step::{PreparedStep, StepFailure, BEFORE_REQUEST_STOPPED};
use super::{RunnableRequest, Variable};
use crate::environment::Environment;
use crate::prepared::PreparedRequest;
use crate::response::{HttpResponse, StatusClass};
use crate::script::{Script, ScriptContext, ScriptReport};
use crate::secret::Redactor;
use crate::settings::RequestSettings;
use crate::transport::HttpTransport;
use crate::variables::VariableScope;

pub const MAX_ITERATIONS: u32 = 1_000;
pub const MAX_DELAY_MS: u64 = 600_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunPacing {
    iterations: u32,
    delay_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunStepResult {
    pub iteration: u32,
    pub name: String,
    pub method: String,
    pub status: Option<u16>,
    pub elapsed: Duration,
    pub checks_passed: usize,
    pub checks_total: usize,
    pub error: Option<String>,
}

pub struct RunStepOutcome {
    pub result: RunStepResult,
    pub report: ScriptReport,
}

impl fmt::Debug for RunStepOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RunStepOutcome")
            .field("result", &self.result)
            .field("write_count", &self.report.writes.len())
            .finish()
    }
}

pub struct RunSession {
    collection_variables: Option<Vec<Variable>>,
    environment: Option<Environment>,
    settings: RequestSettings,
    extra_variables: Vec<(String, String)>,
}

impl Default for RunPacing {
    fn default() -> Self {
        RunPacing {
            iterations: 1,
            delay_ms: 0,
        }
    }
}

impl RunPacing {
    pub fn new(iterations: u32, delay_ms: u64) -> Self {
        RunPacing {
            iterations: iterations.clamp(1, MAX_ITERATIONS),
            delay_ms: delay_ms.min(MAX_DELAY_MS),
        }
    }

    pub fn iterations(&self) -> u32 {
        self.iterations
    }

    pub fn delay(&self) -> Duration {
        Duration::from_millis(self.delay_ms)
    }

    pub fn delay_ms(&self) -> u64 {
        self.delay_ms
    }

    pub fn step_count(&self, requests: usize) -> usize {
        requests.saturating_mul(self.iterations as usize)
    }

    pub fn pauses_after(&self, completed: usize, requests: usize) -> bool {
        self.delay_ms > 0 && completed < self.step_count(requests)
    }
}

impl RunStepResult {
    pub fn is_success(&self) -> bool {
        self.error.is_none()
            && self
                .status
                .is_some_and(|status| !StatusClass::of(status).is_error())
            && self.checks_passed == self.checks_total
    }

    fn responded(
        iteration: u32,
        runnable: &RunnableRequest,
        response: &HttpResponse,
        report: &ScriptReport,
    ) -> Self {
        RunStepResult {
            iteration,
            name: runnable.qualified_name(),
            method: runnable.request.method_name().to_string(),
            status: Some(response.status),
            elapsed: response.elapsed,
            checks_passed: report.passed_count(),
            checks_total: report.checks.len(),
            error: None,
        }
    }

    fn failed(iteration: u32, runnable: &RunnableRequest, error: String) -> Self {
        RunStepResult {
            iteration,
            name: runnable.qualified_name(),
            method: runnable.request.method_name().to_string(),
            status: None,
            elapsed: Duration::ZERO,
            checks_passed: 0,
            checks_total: 0,
            error: Some(error),
        }
    }
}

impl RunSession {
    pub fn new(
        collection_variables: Option<Vec<Variable>>,
        environment: Option<Environment>,
        settings: RequestSettings,
    ) -> Self {
        RunSession {
            collection_variables,
            environment,
            settings,
            extra_variables: Vec::new(),
        }
    }

    pub fn with_variables(mut self, extra_variables: Vec<(String, String)>) -> Self {
        self.extra_variables = extra_variables;
        self
    }

    pub fn prepare(&mut self, runnable: &RunnableRequest) -> Result<PreparedStep, StepFailure> {
        let report = match runnable.scripts.before.runnable() {
            Some(script) => self.run_script(script, runnable, &[], None),
            None => ScriptReport::default(),
        };
        let scope = self.scope(runnable, &report.request_variables());
        let redactor = Redactor::for_request(&scope, runnable.request.auth.as_ref());
        if let Some(failure) = &report.failure {
            let reason = redactor.redact_text(&format!("{BEFORE_REQUEST_STOPPED}{failure}"));
            return Err(StepFailure { reason, report });
        }
        let request = match PreparedRequest::from_request(&runnable.request, &scope, &self.settings)
        {
            Ok(request) => request,
            Err(error) => {
                let reason = redactor.redact_text(&error.to_string());
                return Err(StepFailure { reason, report });
            }
        };
        Ok(PreparedStep {
            request,
            redactor,
            report,
        })
    }

    pub fn send(
        &mut self,
        runnable: &RunnableRequest,
        step: PreparedStep,
        transport: &dyn HttpTransport,
    ) -> Result<(HttpResponse, ScriptReport), StepFailure> {
        let PreparedStep {
            request,
            redactor,
            mut report,
        } = step;
        let response = match transport.send(request) {
            Ok(response) => response,
            Err(error) => {
                let reason = redactor.redact_text(&error.to_string());
                return Err(StepFailure { reason, report });
            }
        };
        if let Some(script) = runnable.scripts.after.runnable() {
            let locals = report.request_variables();
            report.absorb(self.run_script(script, runnable, &locals, Some(&response)));
        }
        Ok((response, report))
    }

    fn run_script(
        &mut self,
        script: &Script,
        runnable: &RunnableRequest,
        locals: &[(String, String)],
        response: Option<&HttpResponse>,
    ) -> ScriptReport {
        let scope = self.scope(runnable, locals);
        let report = script.run(&ScriptContext {
            response,
            variables: &scope,
        });
        report.apply(self.environment.as_mut(), &mut self.collection_variables);
        report
    }

    fn scope(&self, runnable: &RunnableRequest, locals: &[(String, String)]) -> VariableScope {
        let mut overrides = self.extra_variables.clone();
        overrides.extend(locals.iter().cloned());
        VariableScope::layered(
            self.collection_variables.as_deref().unwrap_or_default(),
            &runnable.variables,
            self.environment.as_ref(),
            &overrides,
        )
    }

    pub fn run_one(
        &mut self,
        iteration: u32,
        runnable: &RunnableRequest,
        transport: &dyn HttpTransport,
    ) -> RunStepOutcome {
        let outcome = self
            .prepare(runnable)
            .and_then(|step| self.send(runnable, step, transport));
        match outcome {
            Ok((response, report)) => RunStepOutcome {
                result: RunStepResult::responded(iteration, runnable, &response, &report),
                report,
            },
            Err(failure) => RunStepOutcome {
                result: RunStepResult::failed(iteration, runnable, failure.reason),
                report: failure.report,
            },
        }
    }
}
