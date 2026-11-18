use std::io::Write;

use requestor_core::collection::{RunnableRequest, Variable};
use requestor_core::environment::Environment;
use requestor_core::prepared::PreparedRequest;
use requestor_core::secret::Redactor;
use requestor_core::settings::RequestSettings;
use requestor_core::transport::{HttpTransport, ReqwestTransport};
use requestor_core::variables::VariableScope;

use crate::args::RunOptions;
use crate::error::CliError;
use crate::input::{load_collection, read_text};
use crate::report::{RequestLine, RequestReport};

struct Session<'a> {
    collection_variables: Option<Vec<Variable>>,
    environment: Option<Environment>,
    options: &'a RunOptions,
    settings: RequestSettings,
}

pub fn execute(options: &RunOptions, out: &mut impl Write) -> Result<bool, CliError> {
    let collection = load_collection(&options.collection)?;
    let planned = options.selection.pick(&collection)?;
    let mut session = Session {
        environment: load_environment(options)?,
        settings: options.network.to_settings(),
        collection_variables: collection.variable.clone(),
        options,
    };
    let transport = ReqwestTransport::new(&session.settings)
        .map_err(|error| CliError::input(error.to_string()))?;
    let mut all_succeeded = true;
    for runnable in &planned {
        let report = session.run_one(runnable, &transport);
        all_succeeded &= report.is_success();
        let line = if options.json {
            report.to_json()
        } else {
            report.to_text()
        };
        writeln!(out, "{line}").map_err(|error| CliError::from_write(&error))?;
    }
    Ok(all_succeeded)
}

fn load_environment(options: &RunOptions) -> Result<Option<Environment>, CliError> {
    let Some(path) = &options.environment else {
        return Ok(None);
    };
    Environment::from_json(&read_text(path)?)
        .map(Some)
        .map_err(|error| CliError::input(format!("{}: {error}", path.display())))
}

impl Session<'_> {
    fn scope_for(&self, runnable: &RunnableRequest) -> VariableScope {
        VariableScope::layered(
            self.collection_variables.as_deref().unwrap_or_default(),
            &runnable.variables,
            self.environment.as_ref(),
            &self.options.variables,
        )
    }

    fn run_one(
        &mut self,
        runnable: &RunnableRequest,
        transport: &dyn HttpTransport,
    ) -> RequestReport {
        let name = runnable.qualified_name();
        let scope = self.scope_for(runnable);
        let redactor = Redactor::for_request(&scope, runnable.request.auth.as_ref());
        let prepared =
            match PreparedRequest::from_request(&runnable.request, &scope, &self.settings) {
                Ok(prepared) => prepared,
                Err(error) => {
                    return RequestReport::failed(
                        &name,
                        None,
                        redactor.redact_text(&error.to_string()),
                    )
                }
            };
        warn_unresolved(&name, &prepared);
        let line = RequestLine::from(&prepared, &redactor);
        match transport.send(prepared) {
            Ok(response) => {
                let report = runnable.rules.evaluate(&response);
                report.apply(self.environment.as_mut(), &mut self.collection_variables);
                let checks = report
                    .checks
                    .into_iter()
                    .map(|outcome| outcome.redacted(&redactor))
                    .collect();
                RequestReport::from_response(&name, &line, &response).with_checks(checks)
            }
            Err(error) => {
                RequestReport::failed(&name, Some(&line), redactor.redact_text(&error.to_string()))
            }
        }
    }
}

fn warn_unresolved(name: &str, prepared: &PreparedRequest) {
    let unresolved = prepared.unresolved_variables();
    if !unresolved.is_empty() {
        eprintln!(
            "warning: {name}: unresolved variables: {}",
            unresolved.join(", ")
        );
    }
}
