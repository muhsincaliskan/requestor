use super::RequestDraft;
use crate::import::ImportedRequest;

impl RequestDraft {
    pub fn adopt_pasted_curl(&mut self) -> bool {
        let command = self.url.trim();
        if !ImportedRequest::is_curl_command(command) {
            return false;
        }
        let Ok(imported) = ImportedRequest::from_curl(command) else {
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
        assert!(draft.adopt_pasted_curl());
        assert_eq!(draft.name, "Mine");
        assert_eq!(draft.method, "POST");
        assert_eq!(draft.url, "https://x.test/items?a=1");
        assert_eq!(draft.query[0].key, "a");
        assert_eq!(draft.headers[0].key, "Accept");
        assert!(!draft.is_unchanged());
        assert_eq!(draft.to_request().method.as_deref(), Some("POST"));
    }

    #[test]
    fn ordinary_urls_and_broken_commands_are_left_alone() {
        let mut draft = RequestDraft::blank("r");
        draft.url = "https://x.test/curl".to_string();
        assert!(!draft.adopt_pasted_curl());
        draft.url = "curl 'unterminated".to_string();
        assert!(!draft.adopt_pasted_curl());
        assert_eq!(draft.url, "curl 'unterminated");
    }
}
