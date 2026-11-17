use crate::media_type;

pub const SOAP_ACTION_HEADER: &str = "SOAPAction";

const ENVELOPE_ELEMENT: &str = "Envelope";
const SOAP_1_1_NAMESPACE: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const SOAP_1_2_NAMESPACE: &str = "http://www.w3.org/2003/05/soap-envelope";
const ENVELOPE_TEMPLATE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="{namespace}">
  <soap:Header/>
  <soap:Body>
{content}
  </soap:Body>
</soap:Envelope>"#;
const NAMESPACE_SLOT: &str = "{namespace}";
const CONTENT_SLOT: &str = "{content}";
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

    pub fn envelope(self, content: &str) -> String {
        let indented = content
            .trim()
            .lines()
            .map(|line| format!("{CONTENT_INDENT}{line}"))
            .collect::<Vec<_>>()
            .join("\n");
        ENVELOPE_TEMPLATE
            .replace(NAMESPACE_SLOT, self.namespace())
            .replace(CONTENT_SLOT, &indented)
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

#[cfg(test)]
mod tests {
    use super::SoapVersion;
    use crate::media_type;

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
