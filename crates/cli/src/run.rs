use std::io::Write;

use requestor_core::collection::{RunSession, RunnableRequest};
use requestor_core::environment::Environment;
use requestor_core::prepared::PreparedRequest;
use requestor_core::transport::{HttpTransport, ReqwestTransport};

use crate::args::RunOptions;
use crate::error::CliError;
use crate::input::{load_collection, read_text};
use crate::report::{RequestLine, RequestReport};

pub fn execute(options: &RunOptions, out: &mut impl Write) -> Result<bool, CliError> {
    let collection = load_collection(&options.collection)?;
    let planned = options.selection.pick(&collection)?;
    let settings = options.network.to_settings();
    let mut session = RunSession::new(
        collection.variable.clone(),
        load_environment(options)?,
        settings.clone(),
    )
    .with_variables(options.variables.clone());
    let transport =
        ReqwestTransport::new(&settings).map_err(|error| CliError::input(error.to_string()))?;
    let mut all_succeeded = true;
    for runnable in &planned {
        let report = report_step(&mut session, runnable, &transport);
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

fn report_step(
    session: &mut RunSession,
    runnable: &RunnableRequest,
    transport: &dyn HttpTransport,
) -> RequestReport {
    let name = runnable.qualified_name();
    let (prepared, redactor) = match session.prepare(runnable) {
        Ok(request) => request,
        Err(error) => return RequestReport::failed(&name, None, error),
    };
    warn_unresolved(&name, &prepared);
    let line = RequestLine::from(&prepared, &redactor);
    match session.send(runnable, prepared, &redactor, transport) {
        Ok((response, report)) => {
            let checks = report
                .checks
                .into_iter()
                .map(|outcome| outcome.redacted(&redactor))
                .collect();
            RequestReport::from_response(&name, &line, &response).with_checks(checks)
        }
        Err(error) => RequestReport::failed(&name, Some(&line), error),
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
