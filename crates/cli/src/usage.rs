pub const HELP: &str = "\
Requestor command-line HTTP client

USAGE:
    requestor run <collection.json> [OPTIONS]
    requestor send [OPTIONS] <curl command | raw http file>
    requestor import <file> --to postman
    requestor --help | --version

RUN OPTIONS:
    --env <env.json>        Postman environment file
    --folder <name>         Only run requests inside this folder
    --request <name>        Only run requests with this name
    --var <key=value>       Override a variable (repeatable)
    --json                  Print one JSON object per request

SHARED OPTIONS (run, send):
    --timeout-ms <n>        Request timeout in milliseconds, 0 disables it
    --insecure              Skip TLS certificate verification
    --no-follow-redirects   Do not follow redirects

EXIT CODES:
    0  every request succeeded (2xx or 3xx)
    1  at least one request failed
    2  usage or input error
";

pub const HINT: &str = "Run `requestor --help` for usage.";
