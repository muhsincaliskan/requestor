use crate::media_type;

pub const SOAP_ACTION_HEADER: &str = "SOAPAction";

const ACTION_PARAMETER: &str = "action";
const ENVELOPE_ELEMENT: &str = "Envelope";
const SOAP_1_1_NAMESPACE: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const SOAP_1_2_NAMESPACE: &str = "http://www.w3.org/2003/05/soap-envelope";
const XML_DECLARATION: &str = r#"<?xml version="1.0" encoding="utf-8"?>"#;
const EMPTY_HEADER: &str = "  <soap:Header/>";
const HEADER_START: &str = "  <soap:Header>";
const HEADER_END: &str = "  </soap:Header>";
const CONTENT_INDENT: &str = "    ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SoapVersion {
    #[default]
    V1_1,
    V1_2,
}

impl SoapVersion {
    pub const ALL: [SoapVersion; 2] = [SoapVersion::V1_1, SoapVersion::V1_2];

    pub fn of_envelope(xml: &str) -> Option<Self> {
        if !xml.contains(ENVELOPE_ELEMENT) {
            return None;
        }
        SoapVersion::ALL
            .into_iter()
            .find(|version| xml.contains(version.namespace()))
    }

    pub fn label(self) -> &'static str {
        match self {
            SoapVersion::V1_1 => "SOAP 1.1",
            SoapVersion::V1_2 => "SOAP 1.2",
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            SoapVersion::V1_1 => media_type::SOAP_1_1,
            SoapVersion::V1_2 => media_type::SOAP_1_2,
        }
    }

    pub fn with_action(content_type: String, action: Option<&str>) -> String {
        match action.map(str::trim) {
            Some(action)
                if !action.is_empty() && content_type == SoapVersion::V1_2.content_type() =>
            {
                format!("{content_type}; {ACTION_PARAMETER}=\"{action}\"")
            }
            _ => content_type,
        }
    }

    pub fn envelope(self, content: &str) -> String {
        self.envelope_with_header("", content)
    }

    pub fn envelope_with_header(self, header: &str, content: &str) -> String {
        let header = if header.trim().is_empty() {
            EMPTY_HEADER.to_string()
        } else {
            format!("{HEADER_START}\n{}\n{HEADER_END}", indented(header))
        };
        format!(
            "{XML_DECLARATION}\n<soap:Envelope xmlns:soap=\"{}\">\n{header}\n  <soap:Body>\n{}\n  </soap:Body>\n</soap:Envelope>",
            self.namespace(),
            indented(content)
        )
    }

    pub fn convert(self, xml: &str) -> String {
        match SoapVersion::of_envelope(xml) {
            Some(current) => xml.replace(current.namespace(), self.namespace()),
            None => self.envelope(xml),
        }
    }

    fn namespace(self) -> &'static str {
        match self {
            SoapVersion::V1_1 => SOAP_1_1_NAMESPACE,
            SoapVersion::V1_2 => SOAP_1_2_NAMESPACE,
        }
    }
}

fn indented(content: &str) -> String {
    content
        .trim()
        .lines()
        .map(|line| format!("{CONTENT_INDENT}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::SoapVersion;
    use crate::media_type;

    #[test]
    fn only_soap_1_2_carries_the_action_in_its_content_type() {
        let soap_1_2 = SoapVersion::V1_2.content_type().to_string();
        assert_eq!(
            SoapVersion::with_action(soap_1_2.clone(), Some("urn:Ping")),
            "application/soap+xml; charset=utf-8; action=\"urn:Ping\""
        );
        assert_eq!(
            SoapVersion::with_action(soap_1_2.clone(), Some(" ")),
            soap_1_2
        );
        let soap_1_1 = SoapVersion::V1_1.content_type().to_string();
        assert_eq!(
            SoapVersion::with_action(soap_1_1.clone(), Some("urn:Ping")),
            soap_1_1
        );
    }

    #[test]
    fn header_content_replaces_the_empty_header() {
        let envelope = SoapVersion::V1_1.envelope_with_header("<a/>", "<b/>");
        assert!(envelope
            .contains("  <soap:Header>\n    <a/>\n  </soap:Header>\n  <soap:Body>\n    <b/>\n"));
        assert!(SoapVersion::V1_1
            .envelope("<b/>")
            .contains("  <soap:Header/>\n"));
    }

    #[test]
    fn envelopes_are_recognised_by_their_namespace() {
        for version in SoapVersion::ALL {
            let template = version.envelope("");
            assert_eq!(SoapVersion::of_envelope(&template), Some(version));
        }
        assert_eq!(SoapVersion::of_envelope("<Envelope/>"), None);
        assert_eq!(SoapVersion::of_envelope("<a/>"), None);
    }

    #[test]
    fn converting_swaps_the_namespace_and_keeps_the_content() {
        let original = SoapVersion::V1_1.envelope("<Ping/>");
        let converted = SoapVersion::V1_2.convert(&original);
        assert_eq!(
            SoapVersion::of_envelope(&converted),
            Some(SoapVersion::V1_2)
        );
        assert!(converted.contains("    <Ping/>"));
    }

    #[test]
    fn converting_plain_xml_wraps_it_in_an_envelope() {
        let wrapped = SoapVersion::V1_1.convert("<Ping>\n  <Id>1</Id>\n</Ping>");
        assert_eq!(SoapVersion::of_envelope(&wrapped), Some(SoapVersion::V1_1));
        assert!(wrapped
            .contains("  <soap:Body>\n    <Ping>\n      <Id>1</Id>\n    </Ping>\n  </soap:Body>"));
    }

    #[test]
    fn each_version_has_its_content_type() {
        assert_eq!(SoapVersion::V1_1.content_type(), media_type::SOAP_1_1);
        assert_eq!(SoapVersion::V1_2.content_type(), media_type::SOAP_1_2);
    }
}
