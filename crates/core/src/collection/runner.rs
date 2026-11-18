use std::fmt;
use std::time::Duration;

use super::{RunnableRequest, Variable};
use crate::checks::{Extracted, RulesReport};
use crate::environment::Environment;
use crate::prepared::PreparedRequest;
use crate::response::{HttpResponse, StatusClass};
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
    pub extracted: Vec<Extracted>,
}

impl fmt::Debug for RunStepOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RunStepOutcome")
            .field("result", &self.result)
            .field("extracted_count", &self.extracted.len())
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
        report: &RulesReport,
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

    pub fn prepare(
        &self,
        runnable: &RunnableRequest,
    ) -> Result<(PreparedRequest, Redactor), String> {
        let scope = VariableScope::layered(
            self.collection_variables.as_deref().unwrap_or_default(),
            &runnable.variables,
            self.environment.as_ref(),
            &self.extra_variables,
        );
        let redactor = Redactor::for_request(&scope, runnable.request.auth.as_ref());
        let prepared = PreparedRequest::from_request(&runnable.request, &scope, &self.settings)
            .map_err(|error| redactor.redact_text(&error.to_string()))?;
        Ok((prepared, redactor))
    }

    pub fn send(
        &mut self,
        runnable: &RunnableRequest,
        prepared: PreparedRequest,
        redactor: &Redactor,
        transport: &dyn HttpTransport,
    ) -> Result<(HttpResponse, RulesReport), String> {
        let response = transport
            .send(prepared)
            .map_err(|error| redactor.redact_text(&error.to_string()))?;
        let report = runnable.rules.evaluate(&response);
        report.apply(self.environment.as_mut(), &mut self.collection_variables);
        Ok((response, report))
    }

    pub fn run_one(
        &mut self,
        iteration: u32,
        runnable: &RunnableRequest,
        transport: &dyn HttpTransport,
    ) -> RunStepOutcome {
        let outcome = self
            .prepare(runnable)
            .and_then(|(prepared, redactor)| self.send(runnable, prepared, &redactor, transport));
        match outcome {
            Ok((response, report)) => RunStepOutcome {
                result: RunStepResult::responded(iteration, runnable, &response, &report),
                extracted: report.extracted,
            },
            Err(error) => RunStepOutcome {
                result: RunStepResult::failed(iteration, runnable, error),
                extracted: Vec::new(),
            },
        }
    }
}
