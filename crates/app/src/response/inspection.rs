use requestor_core::json_query::{InvalidQuery, JsonQuery};
use requestor_core::json_tree::JsonNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyLayout {
    Pretty,
    Raw,
    Tree,
}

#[derive(Default)]
pub struct JsonInspection {
    filter_text: String,
    query: Option<Result<JsonQuery, InvalidQuery>>,
    document: Option<Option<JsonNode>>,
    filtered: Option<Option<JsonNode>>,
}

impl BodyLayout {
    pub const ALL: [BodyLayout; 3] = [BodyLayout::Pretty, BodyLayout::Raw, BodyLayout::Tree];

    pub fn label(self) -> &'static str {
        match self {
            BodyLayout::Pretty => "Pretty",
            BodyLayout::Raw => "Raw",
            BodyLayout::Tree => "Tree",
        }
    }

    pub fn is_pretty(self) -> bool {
        self != BodyLayout::Raw
    }
}

impl JsonInspection {
    pub fn filter_text(&self) -> &str {
        &self.filter_text
    }

    pub fn error(&self) -> Option<InvalidQuery> {
        self.query.as_ref()?.as_ref().err().copied()
    }

    pub fn is_filtering(&self) -> bool {
        self.active_query().is_some()
    }

    pub(super) fn set_filter(&mut self, text: String) -> bool {
        if text == self.filter_text {
            return false;
        }
        let trimmed = text.trim();
        self.query = (!trimmed.is_empty()).then(|| JsonQuery::parse(trimmed));
        self.filter_text = text;
        self.filtered = None;
        true
    }

    pub(super) fn result_count(&mut self, body: &[u8]) -> Option<usize> {
        self.filtered(body).map(JsonNode::child_count)
    }

    pub(super) fn filtered_text(&mut self, body: &[u8], pretty: bool) -> Option<String> {
        self.filtered(body).map(|found| found.to_text(pretty))
    }

    pub(super) fn tree(&mut self, body: &[u8]) -> Option<&JsonNode> {
        if self.is_filtering() {
            return self.filtered(body);
        }
        self.document(body)
    }

    pub(super) fn release(&mut self) {
        self.document = None;
        self.filtered = None;
    }

    fn active_query(&self) -> Option<&JsonQuery> {
        self.query.as_ref()?.as_ref().ok()
    }

    fn document(&mut self, body: &[u8]) -> Option<&JsonNode> {
        self.document
            .get_or_insert_with(|| JsonNode::parse(&String::from_utf8_lossy(body)))
            .as_ref()
    }

    fn filtered(&mut self, body: &[u8]) -> Option<&JsonNode> {
        if self.filtered.is_none() {
            let found = self.selected(body);
            self.filtered = Some(found);
        }
        self.filtered.as_ref()?.as_ref()
    }

    fn selected(&mut self, body: &[u8]) -> Option<JsonNode> {
        let query = self.active_query()?.clone();
        let document = self.document(body)?;
        let found = query.select(document).into_iter().cloned().collect();
        Some(JsonNode::Array(found))
    }
}
