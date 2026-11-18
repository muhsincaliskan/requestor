use std::time::Duration;

use super::{RunnableRequest, Variable};
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

pub struct RunSession {
    collection_variables: Option<Vec<Variable>>,
    environment: Option<Environment>,
    settings: RequestSettings,
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
        }
    }

    pub fn run_one(
        &mut self,
        iteration: u32,
        runnable: &RunnableRequest,
        transport: &dyn HttpTransport,
    ) -> RunStepResult {
        let scope = self.scope_for(runnable);
        let redactor = Redactor::for_request(&scope, runnable.request.auth.as_ref());
        let prepared = PreparedRequest::from_request(&runnable.request, &scope, &self.settings);
        let outcome = prepared
            .map_err(|error| error.to_string())
            .and_then(|prepared| transport.send(prepared).map_err(|error| error.to_string()));
        match outcome {
            Ok(response) => self.finished(iteration, runnable, &response),
            Err(error) => RunStepResult::failed(iteration, runnable, redactor.redact_text(&error)),
        }
    }

    fn scope_for(&self, runnable: &RunnableRequest) -> VariableScope {
        VariableScope::layered(
            self.collection_variables.as_deref().unwrap_or_default(),
            &runnable.variables,
            self.environment.as_ref(),
            &[],
        )
    }

    fn finished(
        &mut self,
        iteration: u32,
        runnable: &RunnableRequest,
        response: &HttpResponse,
    ) -> RunStepResult {
        let report = runnable.rules.evaluate(response);
        report.apply(self.environment.as_mut(), &mut self.collection_variables);
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
}
