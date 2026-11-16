use std::path::PathBuf;

use crate::cursor::Cursor;
use crate::error::CliError;
use crate::network::NetworkOptions;
use crate::plan::Selection;

const FLAG_PREFIX: &str = "--";
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

impl Command {
    pub fn parse(arguments: Vec<String>) -> Result<Self, CliError> {
        let mut cursor = Cursor::new(arguments);
        let name = cursor
            .next_item()
            .ok_or_else(|| CliError::usage("missing command"))?;
        match name.as_str() {
            "--help" | "-h" | "help" => Ok(Command::Help),
            "--version" | "-V" => Ok(Command::Version),
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
        while let Some(argument) = cursor.next_item() {
            if network.accept(&argument, &mut cursor)? {
                continue;
            }
            let mut target = vec![argument];
            target.extend(cursor.drain());
            return Ok(SendOptions { target, network });
        }
        Err(CliError::usage(
            "send needs a curl command or a raw http file",
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<Command, CliError> {
        Command::parse(arguments.iter().map(ToString::to_string).collect())
    }

    #[test]
    fn parses_every_run_option() {
        let command = parse(&[
            "run",
            "c.json",
            "--env",
            "e.json",
            "--folder",
            "Users",
            "--request",
            "List",
            "--var",
            "a=1",
            "--var",
            "b=x=y",
            "--timeout-ms",
            "250",
            "--insecure",
            "--no-follow-redirects",
            "--json",
        ])
        .unwrap();
        let Command::Run(options) = command else {
            panic!("expected run")
        };
        assert_eq!(options.collection, PathBuf::from("c.json"));
        assert_eq!(options.environment, Some(PathBuf::from("e.json")));
        assert_eq!(options.selection.folder.as_deref(), Some("Users"));
        assert_eq!(options.selection.request.as_deref(), Some("List"));
        assert_eq!(
            options.variables,
            vec![("a".into(), "1".into()), ("b".into(), "x=y".into())]
        );
        assert_eq!(options.network.timeout_ms, Some(250));
        assert!(options.network.insecure && options.network.no_follow_redirects);
        assert!(options.json);
    }

    #[test]
    fn run_requires_a_collection() {
        assert!(parse(&["run", "--json"]).is_err());
    }

    #[test]
    fn run_rejects_bad_input() {
        assert!(parse(&["run", "c.json", "--bogus"]).is_err());
        assert!(parse(&["run", "a.json", "b.json"]).is_err());
        assert!(parse(&["run", "c.json", "--var", "novalue"]).is_err());
        assert!(parse(&["run", "c.json", "--timeout-ms", "soon"]).is_err());
        assert!(parse(&["run", "c.json", "--env"]).is_err());
    }

    #[test]
    fn send_keeps_curl_arguments_untouched() {
        let command = parse(&["send", "--insecure", "curl", "-X", "POST", "http://h"]).unwrap();
        let Command::Send(options) = command else {
            panic!("expected send")
        };
        assert_eq!(options.target, ["curl", "-X", "POST", "http://h"]);
        assert!(options.network.insecure);
    }

    #[test]
    fn send_requires_a_target() {
        assert!(parse(&["send"]).is_err());
        assert!(parse(&["send", "--insecure"]).is_err());
    }

    #[test]
    fn import_requires_postman_target() {
        assert!(parse(&["import", "f.txt"]).is_err());
        assert!(parse(&["import", "f.txt", "--to", "yaml"]).is_err());
        assert!(parse(&["import", "--to", "postman"]).is_err());
        let command = parse(&["import", "f.txt", "--to", "postman"]).unwrap();
        assert_eq!(
            command,
            Command::Import(ImportOptions {
                file: PathBuf::from("f.txt")
            })
        );
    }

    #[test]
    fn recognizes_help_version_and_unknown_commands() {
        assert_eq!(parse(&["--help"]).unwrap(), Command::Help);
        assert_eq!(parse(&["--version"]).unwrap(), Command::Version);
        assert!(parse(&["fly"]).is_err());
        assert!(parse(&[]).is_err());
    }
}
