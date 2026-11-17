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
fn send_accepts_options_after_the_target() {
    let command = parse(&["send", "curl http://h", "--timeout-ms", "9", "--insecure"]).unwrap();
    let Command::Send(options) = command else {
        panic!("expected send")
    };
    assert_eq!(options.target, ["curl http://h"]);
    assert_eq!(options.network.timeout_ms, Some(9));
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
fn help_works_before_and_after_any_subcommand() {
    for arguments in [
        &["--help"][..],
        &["-h"],
        &["help"],
        &["run", "-h"],
        &["run", "c.json", "--help"],
        &["send", "--help"],
        &["send", "curl http://h", "-h"],
        &["import", "-h"],
    ] {
        assert_eq!(parse(arguments).unwrap(), Command::Help, "{arguments:?}");
    }
}

#[test]
fn recognizes_version_and_unknown_commands() {
    assert_eq!(parse(&["--version"]).unwrap(), Command::Version);
    assert!(parse(&["fly"]).is_err());
    assert!(parse(&[]).is_err());
}
