use std::io::Write;

use requestor_core::import::{import_text, Imported, ImportedRequest};
use requestor_core::prepared::PreparedRequest;
use requestor_core::transport::{HttpTransport, ReqwestTransport};
use requestor_core::variables::VariableScope;

use crate::args::SendOptions;
use crate::error::CliError;
use crate::input::load_send_target;
use crate::report::RequestReport;
use crate::response_view::ResponseView;

const TRUNCATION_NOTICE: &str = "note: response body truncated at the configured size limit";

pub fn execute(options: &SendOptions, out: &mut impl Write) -> Result<bool, CliError> {
    let text = load_send_target(&options.target)?;
    let imported = single_request(&text)?;
    for warning in &imported.warnings {
        eprintln!("warning: {warning}");
    }
    let settings = options.network.to_settings();
    let prepared =
        PreparedRequest::from_request(&imported.request, &VariableScope::new(), &settings)
            .map_err(|error| CliError::input(error.to_string()))?;
    let transport =
        ReqwestTransport::new(&settings).map_err(|error| CliError::input(error.to_string()))?;
    match transport.send(prepared) {
        Ok(response) => {
            let view = ResponseView::from(&response);
            write!(out, "{}", view.to_text())
                .map_err(|error| CliError::input(error.to_string()))?;
            eprintln!("\n{}", view.summary);
            if view.truncated {
                eprintln!("{TRUNCATION_NOTICE}");
            }
            Ok(RequestReport::is_success_status(response.status))
        }
        Err(error) => {
            eprintln!("error: {error}");
            Ok(false)
        }
    }
}

fn single_request(text: &str) -> Result<ImportedRequest, CliError> {
    match import_text(text).map_err(|error| CliError::input(error.to_string()))? {
        Imported::Request(request) => Ok(request),
        Imported::Collection(_) | Imported::Environment(_) => Err(CliError::input(
            "send needs one request; use `requestor run` for collections",
        )),
    }
}
