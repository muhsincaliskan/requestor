const SHORT_FLAGS_WITH_VALUE: &str = "XHdFuAbeoTmxUEwcKCrtzDPQyY";
const SHORT_NAMES: &[(char, &str)] = &[
    ('X', "request"),
    ('H', "header"),
    ('d', "data"),
    ('F', "form"),
    ('u', "user"),
    ('A', "user-agent"),
    ('b', "cookie"),
    ('e', "referer"),
    ('G', "get"),
    ('I', "head"),
    ('k', "insecure"),
    ('s', "silent"),
    ('S', "show-error"),
    ('L', "location"),
    ('v', "verbose"),
    ('i', "include"),
    ('f', "fail"),
    ('o', "output"),
    ('m', "max-time"),
    ('x', "proxy"),
    ('D', "dump-header"),
    ('T', "upload-file"),
    ('w', "write-out"),
    ('E', "cert"),
    ('c', "cookie-jar"),
    ('C', "continue-at"),
    ('K', "config"),
    ('r', "range"),
    ('t', "telnet-option"),
    ('U', "proxy-user"),
    ('z', "time-cond"),
    ('O', "remote-name"),
];
const LONG_OPTIONS_WITH_VALUE: &[&str] = &[
    "request",
    "header",
    "data",
    "data-raw",
    "data-binary",
    "data-ascii",
    "data-urlencode",
    "form",
    "form-string",
    "user",
    "user-agent",
    "cookie",
    "cookie-jar",
    "referer",
    "url",
    "url-query",
    "json",
    "output",
    "output-dir",
    "dump-header",
    "upload-file",
    "max-time",
    "connect-timeout",
    "max-redirs",
    "max-filesize",
    "retry",
    "retry-delay",
    "retry-max-time",
    "limit-rate",
    "resolve",
    "connect-to",
    "oauth2-bearer",
    "proxy",
    "proxy-user",
    "proxy-header",
    "preproxy",
    "noproxy",
    "cacert",
    "capath",
    "cert",
    "cert-type",
    "key",
    "key-type",
    "pass",
    "ciphers",
    "tls-max",
    "write-out",
    "range",
    "interface",
    "local-port",
    "dns-servers",
    "doh-url",
    "unix-socket",
    "aws-sigv4",
    "config",
    "netrc-file",
    "time-cond",
    "continue-at",
    "speed-limit",
    "speed-time",
    "expect100-timeout",
    "request-target",
    "etag-save",
    "etag-compare",
    "alt-svc",
    "hsts",
    "trace",
    "trace-ascii",
    "stderr",
    "variable",
    "proto",
    "proto-redir",
    "parallel-max",
    "rate",
    "happy-eyeballs-timeout-ms",
    "quote",
    "ftp-port",
];

pub(super) fn long_takes_value(name: &str) -> bool {
    LONG_OPTIONS_WITH_VALUE.contains(&name)
}

pub(super) fn short_takes_value(flag: char) -> bool {
    SHORT_FLAGS_WITH_VALUE.contains(flag)
}

pub(super) fn short_to_long(flag: char) -> String {
    SHORT_NAMES
        .iter()
        .find(|(short, _)| *short == flag)
        .map_or_else(|| flag.to_string(), |(_, long)| (*long).to_string())
}

pub(super) fn display_flag(name: &str) -> String {
    if name.chars().count() == 1 {
        format!("-{name}")
    } else {
        format!("--{name}")
    }
}
