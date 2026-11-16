mod args;
mod convert;
mod cursor;
mod error;
mod input;
mod json;
mod network;
mod plan;
mod redacted_url;
mod report;
mod response_view;
mod run;
mod send;
mod usage;

use std::io::{self, Write};
use std::process::ExitCode;

use args::Command;
use error::{CliError, EXIT_REQUEST_FAILED, EXIT_SUCCESS, EXIT_USAGE};

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect();
    match dispatch(arguments, &mut io::stdout().lock()) {
        Ok(true) => ExitCode::from(EXIT_SUCCESS),
        Ok(false) => ExitCode::from(EXIT_REQUEST_FAILED),
        Err(error) => {
            report_error(&error);
            ExitCode::from(EXIT_USAGE)
        }
    }
}

fn dispatch(arguments: Vec<String>, out: &mut impl Write) -> Result<bool, CliError> {
    match Command::parse(arguments)? {
        Command::Help => print_line(out, usage::HELP.trim_end()),
        Command::Version => print_line(out, &format!("requestor {}", env!("CARGO_PKG_VERSION"))),
        Command::Run(options) => run::execute(&options, out),
        Command::Send(options) => send::execute(&options, out),
        Command::Import(options) => convert::execute(&options, out),
    }
}

fn print_line(out: &mut impl Write, text: &str) -> Result<bool, CliError> {
    writeln!(out, "{text}").map_err(|error| CliError::input(error.to_string()))?;
    Ok(true)
}

fn report_error(error: &CliError) {
    eprintln!("error: {error}");
    if error.is_usage() {
        eprintln!("{}", usage::HINT);
    }
}
