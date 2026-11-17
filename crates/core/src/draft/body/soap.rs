use super::{BodyDraft, BodyKind};
use crate::soap::SoapVersion;

impl BodyDraft {
    pub fn switch_to(&mut self, kind: BodyKind) {
        self.kind = kind;
        if kind == BodyKind::Soap && self.raw.trim().is_empty() {
            self.raw = SoapVersion::default().envelope("");
        }
    }

    pub fn soap_version(&self) -> Option<SoapVersion> {
        SoapVersion::of_envelope(&self.raw)
    }

    pub fn use_soap_version(&mut self, version: SoapVersion) -> bool {
        if self.soap_version() == Some(version) {
            return false;
        }
        self.raw = version.convert(&self.raw);
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::collection::{Body, BodyMode};
    use crate::draft::{BodyDraft, BodyKind};
    use crate::soap::SoapVersion;

    #[test]
    fn choosing_soap_on_an_empty_body_starts_from_an_envelope() {
        let mut draft = BodyDraft::default();
        draft.switch_to(BodyKind::Soap);
        assert_eq!(draft.soap_version(), Some(SoapVersion::V1_1));
        let body = draft.to_body().unwrap();
        assert_eq!(body.mode, Some(BodyMode::Raw));
        assert_eq!(body.raw_content_type(), SoapVersion::V1_1.content_type());
    }

    #[test]
    fn soap_bodies_are_saved_as_raw_xml_and_reopen_as_soap() {
        let mut draft = BodyDraft::default();
        draft.switch_to(BodyKind::Soap);
        assert!(draft.use_soap_version(SoapVersion::V1_2));
        assert!(!draft.use_soap_version(SoapVersion::V1_2));
        let body = draft.to_body().unwrap();
        let language = body.options.as_ref().unwrap()["raw"]["language"].clone();
        assert_eq!(language, "xml");
        let reopened = BodyDraft::from_body(Some(&body));
        assert_eq!(reopened.kind, BodyKind::Soap);
        assert_eq!(reopened.soap_version(), Some(SoapVersion::V1_2));
    }

    #[test]
    fn plain_xml_stays_raw() {
        let draft = BodyDraft::from_body(Some(&Body::raw("<a/>", Some("xml"))));
        assert_eq!(draft.kind, BodyKind::Raw);
    }
}
