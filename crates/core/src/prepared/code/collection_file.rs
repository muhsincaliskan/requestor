use super::CodeLanguage;
use crate::collection::{Collection, Variable};
use crate::prepared::PreparedRequest;
use crate::settings::RequestSettings;
use crate::variables::VariableScope;

const REQUEST_SEPARATOR: &str = "###";
const LINE_BREAK: &str = "\n";
const SKIPPED_PREFIX: &str = "# Not exported:";

impl Collection {
    pub fn to_http_file(&self, settings: &RequestSettings) -> String {
        let settings = RequestSettings {
            send_postman_token_header: false,
            ..settings.clone()
        };
        let scope = VariableScope::default();
        let mut file: String = self
            .variable
            .iter()
            .flatten()
            .filter_map(file_variable)
            .collect();
        if !file.is_empty() {
            file.push_str(LINE_BREAK);
        }
        let requests: Vec<String> = self
            .runnable_requests()
            .iter()
            .map(|runnable| {
                let code = match PreparedRequest::referencing_files(
                    &runnable.request,
                    &scope,
                    &settings,
                ) {
                    Ok(prepared) => prepared.to_code_with_secrets(CodeLanguage::Http),
                    Err(error) => format!("{SKIPPED_PREFIX} {error}"),
                };
                let name = runnable.qualified_name();
                format!("{REQUEST_SEPARATOR} {name}{LINE_BREAK}{code}{LINE_BREAK}")
            })
            .collect();
        file.push_str(&requests.join(LINE_BREAK));
        file
    }
}

fn file_variable(variable: &Variable) -> Option<String> {
    let value = variable.value_text();
    let usable = variable.is_enabled()
        && !variable.is_secret()
        && !variable.key_text().is_empty()
        && !value.contains(LINE_BREAK);
    usable.then(|| format!("@{} = {value}{LINE_BREAK}", variable.key_text()))
}
