use super::{KeyValueRow, RequestDraft};
use crate::soap::SOAP_ACTION_HEADER;

impl RequestDraft {
    pub fn soap_action(&self) -> &str {
        self.soap_action_row()
            .map(|index| self.headers[index].value.as_str())
            .unwrap_or_default()
    }

    pub fn set_soap_action(&mut self, action: &str) {
        match self.soap_action_row() {
            Some(index) if action.is_empty() => {
                self.headers.remove(index);
            }
            Some(index) => {
                let row = &mut self.headers[index];
                row.value = action.to_string();
                row.enabled = true;
            }
            None if !action.is_empty() => self
                .headers
                .push(KeyValueRow::new(SOAP_ACTION_HEADER, action)),
            None => {}
        }
    }

    fn soap_action_row(&self) -> Option<usize> {
        self.headers
            .iter()
            .position(|row| row.key.trim().eq_ignore_ascii_case(SOAP_ACTION_HEADER))
    }
}

#[cfg(test)]
mod tests {
    use crate::draft::{BodyKind, RequestDraft};
    use crate::prepared::PreparedRequest;
    use crate::settings::RequestSettings;
    use crate::soap::{SoapVersion, SOAP_ACTION_HEADER};
    use crate::variables::{VariableLayer, VariableScope};

    #[test]
    fn the_soap_action_lives_in_the_headers() {
        let mut draft = RequestDraft::blank("r");
        draft.set_soap_action("urn:Ping");
        assert_eq!(draft.soap_action(), "urn:Ping");
        assert_eq!(draft.headers[0].key, SOAP_ACTION_HEADER);
        draft.headers[0].enabled = false;
        draft.set_soap_action("urn:Pong");
        assert!(draft.headers[0].enabled);
        assert_eq!(draft.headers.len(), 1);
        draft.set_soap_action("");
        assert!(draft.headers.is_empty());
    }

    #[test]
    fn a_soap_request_is_sent_with_its_envelope_action_and_content_type() {
        let mut draft = RequestDraft::blank("r");
        draft.method = "POST".to_string();
        draft.url = "{{host}}/service".to_string();
        draft.body.switch_to(BodyKind::Soap);
        draft.set_soap_action("{{action}}");
        let scope = VariableScope::new().with_layer(
            VariableLayer::default()
                .with("host", "https://soap.test")
                .with("action", "urn:Ping"),
        );
        let prepared =
            PreparedRequest::from_request(&draft.to_request(), &scope, &RequestSettings::default())
                .unwrap();
        assert_eq!(prepared.header(SOAP_ACTION_HEADER), Some("urn:Ping"));
        assert_eq!(
            prepared.header("Content-Type"),
            Some(SoapVersion::V1_1.content_type())
        );
        let sent = String::from_utf8(prepared.body.unwrap()).unwrap();
        assert_eq!(SoapVersion::of_envelope(&sent), Some(SoapVersion::V1_1));
    }

    #[test]
    fn an_explicit_content_type_header_wins() {
        let mut draft = RequestDraft::blank("r");
        draft.url = "https://soap.test".to_string();
        draft.body.switch_to(BodyKind::Soap);
        draft
            .headers
            .push(crate::draft::KeyValueRow::new("Content-Type", "text/xml"));
        let prepared = PreparedRequest::from_request(
            &draft.to_request(),
            &VariableScope::new(),
            &RequestSettings::default(),
        )
        .unwrap();
        assert_eq!(prepared.header("Content-Type"), Some("text/xml"));
    }
}
