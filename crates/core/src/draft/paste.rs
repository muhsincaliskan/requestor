use super::RequestDraft;
use crate::import::ImportedRequest;

impl RequestDraft {
    pub fn adopt_pasted_curl(&mut self, pasted: &str) -> bool {
        let Some(imported) = [pasted, self.url.as_str()]
            .into_iter()
            .map(str::trim)
            .filter(|command| ImportedRequest::is_curl_command(command))
            .find_map(|command| ImportedRequest::from_curl(command).ok())
        else {
            return false;
        };
        let mut pasted = RequestDraft::from_request(self.name.clone(), &imported.request);
        pasted.source = self.source.clone();
        *self = pasted;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::RequestDraft;

    #[test]
    fn a_pasted_curl_command_becomes_the_request() {
        let mut draft = RequestDraft::blank("Mine");
        draft.url =
            "curl -X POST https://x.test/items?a=1 -H 'Accept: application/json' -d '{\"k\":1}'"
                .to_string();
        assert!(draft.adopt_pasted_curl(""));
        assert_eq!(draft.name, "Mine");
        assert_eq!(draft.method, "POST");
        assert_eq!(draft.url, "https://x.test/items?a=1");
        assert_eq!(draft.query[0].key, "a");
        assert_eq!(draft.headers[0].key, "Accept");
        assert!(!draft.is_unchanged());
        assert_eq!(draft.to_request().method.as_deref(), Some("POST"));
    }

    #[test]
    fn a_multi_line_command_is_read_from_the_pasted_text() {
        let pasted = "curl --location 'https://x.test/login' \\\n--header 'Content-Type: application/json' \\\n--data '{\"u\":1}'";
        let mut draft = RequestDraft::blank("r");
        draft.url = pasted.replace('\n', " ");
        assert!(draft.adopt_pasted_curl(pasted));
        assert_eq!(draft.url, "https://x.test/login");
        assert_eq!(draft.headers.len(), 1);
        assert_eq!(draft.headers[0].key, "Content-Type");
        assert_eq!(draft.method, "POST");
    }

    #[test]
    fn a_command_with_typographic_quotes_is_understood() {
        let pasted = "curl --location \u{2018}https://auth.test/token\u{2019} \\\n--header \u{2018}Content-Type: application/x-www-form-urlencoded\u{2019} \\\n--data-urlencode \u{2018}grant_type=password\u{2019} \\\n--data-urlencode \u{2018}username=user\u{2019}";
        let mut draft = RequestDraft::blank("r");
        draft.url = pasted.replace('\n', " ");
        assert!(draft.adopt_pasted_curl(pasted));
        assert_eq!(draft.url, "https://auth.test/token");
        assert_eq!(draft.method, "POST");
        assert_eq!(draft.headers[0].value, "application/x-www-form-urlencoded");
        let request = draft.to_request();
        let body = serde_json::to_string(&request.body).unwrap();
        assert!(
            body.contains("grant_type") && body.contains("username"),
            "{body}"
        );
    }

    #[test]
    fn ordinary_urls_and_broken_commands_are_left_alone() {
        let mut draft = RequestDraft::blank("r");
        draft.url = "https://x.test/curl".to_string();
        assert!(!draft.adopt_pasted_curl("https://x.test/curl"));
        draft.url = "curl 'unterminated".to_string();
        assert!(!draft.adopt_pasted_curl("curl 'unterminated"));
        assert_eq!(draft.url, "curl 'unterminated");
    }
}
