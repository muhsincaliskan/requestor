use requestor_core::json_query::{InvalidQuery, JsonQuery};
use requestor_core::json_tree::{JsonKey, JsonNode, NodePath};

const PRETTY_LABEL: &str = "Pretty";
const RAW_LABEL: &str = "Raw";
const TREE_LABEL: &str = "Tree";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyLayout {
    Pretty,
    Raw,
    Tree,
}

#[derive(Default)]
enum ParsedBody {
    #[default]
    NotParsed,
    Unparseable,
    Parsed(JsonNode),
}

pub enum JsonTreeView<'a> {
    Whole(&'a JsonNode),
    Matches(&'a JsonNode, &'a [NodePath]),
}

#[derive(Default)]
pub struct JsonInspection {
    filter_text: String,
    query: Option<JsonQuery>,
    error: Option<InvalidQuery>,
    document: ParsedBody,
    matches: Option<Vec<NodePath>>,
}

impl BodyLayout {
    pub const ALL: [BodyLayout; 3] = [BodyLayout::Pretty, BodyLayout::Raw, BodyLayout::Tree];

    pub fn label(self) -> &'static str {
        match self {
            BodyLayout::Pretty => PRETTY_LABEL,
            BodyLayout::Raw => RAW_LABEL,
            BodyLayout::Tree => TREE_LABEL,
        }
    }

    pub fn is_pretty(self) -> bool {
        self != BodyLayout::Raw
    }
}

impl<'a> JsonTreeView<'a> {
    pub fn summary(&self) -> String {
        match self {
            JsonTreeView::Whole(root) => root.summary(),
            JsonTreeView::Matches(_, paths) => JsonNode::list_summary(paths.len()),
        }
    }

    pub fn child_count(&self) -> usize {
        match self {
            JsonTreeView::Whole(root) => root.child_count(),
            JsonTreeView::Matches(_, paths) => paths.len(),
        }
    }

    pub fn child(&self, position: usize) -> Option<(JsonKey<'a>, &'a JsonNode)> {
        match self {
            JsonTreeView::Whole(root) => root.child(position),
            JsonTreeView::Matches(root, paths) => {
                let found = root.at_path(paths.get(position)?)?;
                Some((JsonKey::Index(position), found))
            }
        }
    }
}

impl JsonInspection {
    pub fn filter_text(&self) -> &str {
        &self.filter_text
    }

    pub fn error(&self) -> Option<InvalidQuery> {
        self.error
    }

    pub fn is_filtering(&self) -> bool {
        self.query.is_some()
    }

    pub(super) fn set_filter(&mut self, text: String) -> bool {
        let parsed = if text.trim().is_empty() {
            Ok(None)
        } else {
            JsonQuery::parse(&text).map(Some)
        };
        self.filter_text = text;
        match parsed {
            Ok(query) => {
                self.error = None;
                self.use_query(query)
            }
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }

    fn use_query(&mut self, query: Option<JsonQuery>) -> bool {
        if query == self.query {
            return false;
        }
        self.query = query;
        self.matches = None;
        true
    }

    pub(super) fn result_count(&mut self, body: &[u8]) -> Option<usize> {
        self.matches(body).map(<[NodePath]>::len)
    }

    pub(super) fn filtered_text(&mut self, body: &[u8], pretty: bool) -> Option<String> {
        self.matches(body)?;
        let JsonTreeView::Matches(root, paths) = self.view()? else {
            return None;
        };
        let found: Vec<&JsonNode> = paths.iter().filter_map(|path| root.at_path(path)).collect();
        Some(JsonNode::list_text(&found, pretty))
    }

    pub(super) fn tree(&mut self, body: &[u8]) -> Option<JsonTreeView<'_>> {
        if self.is_filtering() {
            self.matches(body)?;
        } else {
            self.document(body)?;
        }
        self.view()
    }

    pub(super) fn release(&mut self) {
        self.document = ParsedBody::NotParsed;
        self.matches = None;
    }

    fn view(&self) -> Option<JsonTreeView<'_>> {
        let ParsedBody::Parsed(root) = &self.document else {
            return None;
        };
        Some(match &self.matches {
            Some(paths) if self.is_filtering() => JsonTreeView::Matches(root, paths),
            _ => JsonTreeView::Whole(root),
        })
    }

    fn document(&mut self, body: &[u8]) -> Option<&JsonNode> {
        if matches!(self.document, ParsedBody::NotParsed) {
            self.document = match JsonNode::parse(&String::from_utf8_lossy(body)) {
                Some(root) => ParsedBody::Parsed(root),
                None => ParsedBody::Unparseable,
            };
        }
        match &self.document {
            ParsedBody::Parsed(root) => Some(root),
            _ => None,
        }
    }

    fn matches(&mut self, body: &[u8]) -> Option<&[NodePath]> {
        if self.matches.is_none() {
            let query = self.query.clone()?;
            let paths = query.select_paths(self.document(body)?);
            self.matches = Some(paths);
        }
        self.matches.as_deref()
    }
}
