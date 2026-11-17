use super::{BodyDraft, BodyKind, RawLanguage};
use crate::{json_layout, xml_layout};

impl BodyDraft {
    pub fn can_beautify(&self) -> bool {
        matches!(self.kind, BodyKind::Raw | BodyKind::Soap)
            && matches!(self.raw_language(), RawLanguage::Json | RawLanguage::Xml)
    }

    pub fn beautify(&mut self) -> bool {
        if !self.can_beautify() {
            return false;
        }
        let laid_out = match self.raw_language() {
            RawLanguage::Json => json_layout::reindent(&self.raw),
            _ => xml_layout::reindent(&self.raw),
        };
        if laid_out == self.raw {
            return false;
        }
        self.raw = laid_out;
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::collection::Body;
    use crate::draft::{BodyDraft, BodyKind};

    fn json_draft(raw: &str) -> BodyDraft {
        BodyDraft::from_body(Some(&Body::raw(raw, Some("json"))))
    }

    #[test]
    fn json_bodies_are_laid_out_keeping_key_order_and_variables() {
        let mut draft = json_draft(r#"{"z":1,"a":{{id}}}"#);
        assert!(draft.beautify());
        assert_eq!(draft.raw, "{\n  \"z\": 1,\n  \"a\": {{id}}\n}");
        assert!(!draft.beautify());
    }

    #[test]
    fn xml_bodies_are_laid_out() {
        let mut draft = BodyDraft::from_body(Some(&Body::raw("<a><b>{{id}}</b></a>", Some("xml"))));
        assert!(draft.beautify());
        assert_eq!(draft.raw, "<a>\n  <b>{{id}}</b>\n</a>");
    }

    #[test]
    fn other_languages_and_modes_are_left_alone() {
        let mut text = BodyDraft::from_body(Some(&Body::raw("{\"a\":1}", Some("text"))));
        assert!(!text.can_beautify());
        assert!(!text.beautify());
        let mut switched = json_draft("{\"a\":1}");
        switched.kind = BodyKind::Urlencoded;
        assert!(!switched.beautify());
        assert_eq!(switched.raw, "{\"a\":1}");
    }
}
