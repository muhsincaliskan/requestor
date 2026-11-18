#[cfg(test)]
mod tests;

use std::path::PathBuf;

use requestor_core::import::is_web_address;

use crate::cursor::Cursor;
use crate::error::CliError;
use crate::network::NetworkOptions;
use crate::plan::Selection;

const FLAG_PREFIX: &str = "--";
const HELP_FLAGS: [&str; 2] = ["--help", "-h"];
const ENV_FLAG: &str = "--env";
const FOLDER_FLAG: &str = "--folder";
const REQUEST_FLAG: &str = "--request";
const VAR_FLAG: &str = "--var";
const JSON_FLAG: &str = "--json";
const TO_FLAG: &str = "--to";
const POSTMAN_FORMAT: &str = "postman";
const ASSIGNMENT_SEPARATOR: char = '=';

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    Run(RunOptions),
    Send(SendOptions),
    Import(ImportOptions),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RunOptions {
    pub collection: PathBuf,
    pub environment: Option<PathBuf>,
    pub selection: Selection,
    pub variables: Vec<(String, String)>,
    pub network: NetworkOptions,
    pub json: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SendOptions {
    pub target: Vec<String>,
    pub network: NetworkOptions,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ImportOptions {
    pub file: PathBuf,
}

impl ImportOptions {
    pub fn web_address(&self) -> Option<&str> {
        self.file.to_str().filter(|text| is_web_address(text))
    }
}

impl Command {
    pub fn parse(arguments: Vec<String>) -> Result<Self, CliError> {
        let mut cursor = Cursor::new(arguments);
        let name = cursor
            .next_item()
            .ok_or_else(|| CliError::usage("missing command"))?;
        if HELP_FLAGS.contains(&name.as_str()) || name == "help" {
            return Ok(Command::Help);
        }
        match name.as_str() {
            "--version" | "-V" => Ok(Command::Version),
            "run" | "send" | "import" if cursor.contains_any(&HELP_FLAGS) => Ok(Command::Help),
            "run" => RunOptions::parse(cursor).map(Command::Run),
            "send" => SendOptions::parse(cursor).map(Command::Send),
            "import" => ImportOptions::parse(cursor).map(Command::Import),
            other => Err(CliError::usage(format!("unknown command {other:?}"))),
        }
    }
}

impl RunOptions {
    fn parse(mut cursor: Cursor) -> Result<Self, CliError> {
        let mut options = RunOptions::default();
        let mut collection = None;
        while let Some(argument) = cursor.next_item() {
            if options.accept(&argument, &mut cursor)? {
                continue;
            }
            reject_unknown_flag(&argument)?;
            if collection.replace(PathBuf::from(&argument)).is_some() {
                return Err(unexpected(&argument));
            }
        }
        options.collection =
            collection.ok_or_else(|| CliError::usage("run needs a collection file"))?;
        Ok(options)
    }

    fn accept(&mut self, flag: &str, cursor: &mut Cursor) -> Result<bool, CliError> {
        match flag {
            ENV_FLAG => self.environment = Some(PathBuf::from(cursor.value(flag)?)),
            FOLDER_FLAG => self.selection.folder = Some(cursor.value(flag)?),
            REQUEST_FLAG => self.selection.request = Some(cursor.value(flag)?),
            VAR_FLAG => self.variables.push(parse_assignment(&cursor.value(flag)?)?),
            JSON_FLAG => self.json = true,
            _ => return self.network.accept(flag, cursor),
        }
        Ok(true)
    }
}

impl SendOptions {
    fn parse(mut cursor: Cursor) -> Result<Self, CliError> {
        let mut network = NetworkOptions::default();
        let mut target = Vec::new();
        while let Some(argument) = cursor.next_item() {
            if !network.accept(&argument, &mut cursor)? {
                target.push(argument);
            }
        }
        if target.is_empty() {
            return Err(CliError::usage(
                "send needs a curl command or a raw http file",
            ));
        }
        Ok(SendOptions { target, network })
    }
}

impl ImportOptions {
    fn parse(mut cursor: Cursor) -> Result<Self, CliError> {
        let mut file = None;
        let mut format = None;
        while let Some(argument) = cursor.next_item() {
            if argument == TO_FLAG {
                format = Some(cursor.value(TO_FLAG)?);
                continue;
            }
            reject_unknown_flag(&argument)?;
            if file.replace(PathBuf::from(&argument)).is_some() {
                return Err(unexpected(&argument));
            }
        }
        require_postman(format.as_deref())?;
        let file = file.ok_or_else(|| CliError::usage("import needs a file"))?;
        Ok(ImportOptions { file })
    }
}

fn require_postman(format: Option<&str>) -> Result<(), CliError> {
    match format {
        Some(POSTMAN_FORMAT) => Ok(()),
        Some(other) => Err(CliError::usage(format!(
            "unsupported target format {other:?}"
        ))),
        None => Err(CliError::usage("import needs --to postman")),
    }
}

fn unexpected(argument: &str) -> CliError {
    CliError::usage(format!("unexpected argument {argument:?}"))
}

fn reject_unknown_flag(argument: &str) -> Result<(), CliError> {
    if argument.starts_with(FLAG_PREFIX) {
        return Err(CliError::usage(format!("unknown option {argument:?}")));
    }
    Ok(())
}

fn parse_assignment(text: &str) -> Result<(String, String), CliError> {
    match text.split_once(ASSIGNMENT_SEPARATOR) {
        Some((key, value)) if !key.is_empty() => Ok((key.to_string(), value.to_string())),
        _ => Err(CliError::usage(format!(
            "{VAR_FLAG} needs key=value, got {text:?}"
        ))),
    }
}
