use std::io::Write;

use requestor_core::collection::Collection;
use requestor_core::environment::Environment;
use requestor_core::prepared::PreparedRequest;
use requestor_core::settings::RequestSettings;
use requestor_core::transport::{HttpTransport, ReqwestTransport};
use requestor_core::variables::{VariableLayer, VariableScope};

use crate::args::RunOptions;
use crate::error::CliError;
use crate::input::read_text;
use crate::plan::PlannedRequest;
use crate::report::{RequestLine, RequestReport};

pub fn execute(options: &RunOptions, out: &mut impl Write) -> Result<bool, CliError> {
    let collection = Collection::from_json(&read_text(&options.collection)?)
        .map_err(|error| invalid_file(options, error))?;
    let scope = build_scope(options, &collection)?;
    let planned = PlannedRequest::select(&collection, &options.selection)?;
    let settings = options.network.to_settings();
    let transport =
        ReqwestTransport::new(&settings).map_err(|error| CliError::input(error.to_string()))?;
    let mut all_succeeded = true;
    for entry in &planned {
        let report = run_one(entry, &scope, &settings, &transport);
        all_succeeded &= report.is_success();
        let line = if options.json {
            report.to_json()
        } else {
            report.to_text()
        };
        writeln!(out, "{line}").map_err(|error| CliError::input(error.to_string()))?;
    }
    Ok(all_succeeded)
}

fn invalid_file(options: &RunOptions, error: impl std::fmt::Display) -> CliError {
    CliError::input(format!("{}: {error}", options.collection.display()))
}

fn build_scope(options: &RunOptions, collection: &Collection) -> Result<VariableScope, CliError> {
    let mut scope = VariableScope::new();
    if let Some(variables) = &collection.variable {
        scope = scope.with_layer(VariableLayer::from_variables(variables));
    }
    if let Some(path) = &options.environment {
        let environment = Environment::from_json(&read_text(path)?)
            .map_err(|error| CliError::input(format!("{}: {error}", path.display())))?;
        scope = scope.with_layer(VariableLayer::from_environment(&environment));
    }
    let overrides = options
        .variables
        .iter()
        .fold(VariableLayer::default(), |layer, (key, value)| {
            layer.with(key, value)
        });
    Ok(scope.with_layer(overrides))
}

fn run_one(
    entry: &PlannedRequest,
    scope: &VariableScope,
    settings: &RequestSettings,
    transport: &dyn HttpTransport,
) -> RequestReport {
    let prepared = match PreparedRequest::from_request(&entry.request, scope, settings) {
        Ok(prepared) => prepared,
        Err(error) => return RequestReport::failed(&entry.name, None, error),
    };
    warn_unresolved(&entry.name, &prepared);
    let line = RequestLine::from(&prepared);
    match transport.send(prepared) {
        Ok(response) => RequestReport::from_response(&entry.name, &line, &response),
        Err(error) => RequestReport::failed(&entry.name, Some(&line), error),
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
