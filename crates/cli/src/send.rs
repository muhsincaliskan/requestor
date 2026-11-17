use std::io::Write;

use requestor_core::import::{import_text, ImportedRequest};
use requestor_core::prepared::PreparedRequest;
use requestor_core::secret::Redactor;
use requestor_core::transport::{HttpTransport, ReqwestTransport};
use requestor_core::variables::VariableScope;

use crate::args::SendOptions;
use crate::error::CliError;
use crate::input::load_send_target;
use crate::response_view::ResponseView;

const TRUNCATION_NOTICE: &str = "note: response body truncated at the configured size limit";

pub fn execute(options: &SendOptions, out: &mut impl Write) -> Result<bool, CliError> {
    let imported = import_text(&load_send_target(&options.target)?)
        .map_err(|error| CliError::input(error.to_string()))?;
    for warning in imported.warnings() {
        eprintln!("warning: {warning}");
    }
    let imported =
        ImportedRequest::try_from(imported).map_err(|error| CliError::input(error.to_string()))?;
    let scope = VariableScope::new();
    let redactor = Redactor::for_request(&scope, imported.request.auth.as_ref());
    let settings = options.network.to_settings();
    let prepared = match PreparedRequest::from_request(&imported.request, &scope, &settings) {
        Ok(prepared) => prepared,
        Err(error) => return Ok(report_failure(&redactor, &error)),
    };
    let transport =
        ReqwestTransport::new(&settings).map_err(|error| CliError::input(error.to_string()))?;
    match transport.send(prepared) {
        Ok(response) => {
            let view = ResponseView::from(&response);
            write!(out, "{}", view.to_text()).map_err(|error| CliError::from_write(&error))?;
            eprintln!("\n{}", view.summary);
            if view.truncated {
                eprintln!("{TRUNCATION_NOTICE}");
            }
            Ok(!response.is_client_or_server_error())
        }
        Err(error) => Ok(report_failure(&redactor, &error)),
    }
}

fn report_failure(redactor: &Redactor, error: &impl ToString) -> bool {
    eprintln!("error: {}", redactor.redact_text(&error.to_string()));
    false
}
