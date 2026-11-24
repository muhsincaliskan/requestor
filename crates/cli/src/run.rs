use std::io::Write;

use requestor_core::collection::{RunSession, RunnableRequest};
use requestor_core::environment::Environment;
use requestor_core::prepared::PreparedRequest;
use requestor_core::script::StoredScript;
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
    warn_unconverted(&name, runnable);
    let step = match session.prepare(runnable) {
        Ok(step) => step,
        Err(failure) => return RequestReport::failed(&name, None, failure.reason),
    };
    warn_unresolved(&name, &step.request);
    let line = RequestLine::from(&step.request, &step.redactor);
    let redactor = step.redactor.clone();
    match session.send(runnable, step, transport) {
        Ok((response, report)) => {
            let checks = report
                .checks
                .into_iter()
                .map(|outcome| outcome.redacted(&redactor))
                .collect();
            RequestReport::from_response(&name, &line, &response).with_checks(checks)
        }
        Err(failure) => RequestReport::failed(&name, Some(&line), failure.reason),
    }
}

fn warn_unconverted(name: &str, runnable: &RunnableRequest) {
    for (label, stored) in [
        ("before request", &runnable.scripts.before),
        ("after response", &runnable.scripts.after),
    ] {
        if let StoredScript::Unconverted { problems, .. } = stored {
            let lines: Vec<String> = problems
                .lines
                .iter()
                .map(|line| line.line.to_string())
                .collect();
            eprintln!(
                "warning: {name}: {label} script not run, lines Requestor cannot convert: {}",
                lines.join(", ")
            );
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
