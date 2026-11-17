pub(crate) const COMMAND_NAME: &str = "curl";
const SINGLE_QUOTE_ESCAPE: &str = "'\\''";

pub fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', SINGLE_QUOTE_ESCAPE))
}
