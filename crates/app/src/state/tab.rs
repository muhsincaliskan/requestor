use requestor_core::collection::{Item, Request};
use requestor_core::draft::RequestDraft;
use requestor_core::workspace::{SessionTab, Workspace};

use crate::response::ResponseView;

pub const UNTITLED_REQUEST: &str = "Untitled request";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TabId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemBinding {
    pub collection_id: String,
    pub item_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorSection {
    #[default]
    Params,
    Headers,
    Body,
    Auth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseSection {
    #[default]
    Body,
    Headers,
    Cookies,
}

pub enum Exchange {
    Idle,
    Sending { job: u64 },
    Completed(Box<ResponseView>),
    Failed(String),
}

pub struct RequestTab {
    pub id: TabId,
    pub binding: Option<ItemBinding>,
    pub draft: RequestDraft,
    pub unsaved: bool,
    pub edited_at: Option<f64>,
    pub exchange: Exchange,
    pub editor_section: EditorSection,
    pub response_section: ResponseSection,
}

impl EditorSection {
    pub const ALL: [EditorSection; 4] = [
        EditorSection::Params,
        EditorSection::Headers,
        EditorSection::Body,
        EditorSection::Auth,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EditorSection::Params => "Params",
            EditorSection::Headers => "Headers",
            EditorSection::Body => "Body",
            EditorSection::Auth => "Auth",
        }
    }
}

impl ResponseSection {
    pub const ALL: [ResponseSection; 3] = [
        ResponseSection::Body,
        ResponseSection::Headers,
        ResponseSection::Cookies,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ResponseSection::Body => "Body",
            ResponseSection::Headers => "Headers",
            ResponseSection::Cookies => "Cookies",
        }
    }
}

impl ItemBinding {
    pub fn new(collection_id: impl Into<String>, item_id: impl Into<String>) -> Self {
        ItemBinding {
            collection_id: collection_id.into(),
            item_id: item_id.into(),
        }
    }

    pub fn points_at(&self, collection_id: &str, item_id: &str) -> bool {
        self.collection_id == collection_id && self.item_id == item_id
    }
}

impl RequestTab {
    pub fn scratch(id: TabId) -> Self {
        RequestTab::with_draft(id, None, RequestDraft::blank(UNTITLED_REQUEST), true)
    }

    pub fn for_request(id: TabId, name: &str, request: &Request) -> Self {
        RequestTab::with_draft(id, None, RequestDraft::from_request(name, request), true)
    }

    pub fn for_item(id: TabId, binding: ItemBinding, item: &Item) -> Option<Self> {
        let request = item.full_request()?;
        let draft = RequestDraft::from_request(item.display_name(), &request);
        Some(RequestTab::with_draft(id, Some(binding), draft, false))
    }

    pub fn from_session(id: TabId, saved: &SessionTab, workspace: &Workspace) -> Self {
        let binding = saved
            .collection_id
            .as_deref()
            .zip(saved.item_id.as_deref())
            .filter(|(collection, item)| {
                workspace
                    .collection(collection)
                    .is_some_and(|found| found.find_item(item).is_some())
            })
            .map(|(collection, item)| ItemBinding::new(collection, item));
        let stored = binding.as_ref().and_then(|binding| {
            workspace
                .collection(&binding.collection_id)?
                .find_item(&binding.item_id)?
                .full_request()
        });
        let unsaved = saved.unsaved_request.is_some() || stored.is_none();
        let draft = match saved.unsaved_request.as_ref().or(stored.as_ref()) {
            Some(request) => RequestDraft::from_request(saved.name.as_str(), request),
            None => RequestDraft::blank(saved.name.as_str()),
        };
        RequestTab::with_draft(id, binding, draft, unsaved)
    }

    pub fn to_session(&self) -> SessionTab {
        SessionTab {
            collection_id: self
                .binding
                .as_ref()
                .map(|bound| bound.collection_id.clone()),
            item_id: self.binding.as_ref().map(|bound| bound.item_id.clone()),
            name: self.draft.name.clone(),
            unsaved_request: self.unsaved.then(|| self.draft.to_request()),
        }
    }

    pub fn mark_edited(&mut self, now: f64) {
        self.unsaved = true;
        self.edited_at = Some(now);
    }

    pub fn mark_saved(&mut self) {
        self.unsaved = false;
        self.edited_at = None;
    }

    pub fn is_due_for_autosave(&self, now: f64, quiet_seconds: f64) -> bool {
        self.unsaved
            && self.binding.is_some()
            && self
                .edited_at
                .is_some_and(|edited| now - edited >= quiet_seconds)
    }

    pub fn is_bound_to(&self, collection_id: &str, item_id: &str) -> bool {
        self.binding
            .as_ref()
            .is_some_and(|bound| bound.points_at(collection_id, item_id))
    }

    pub fn is_sending(&self) -> bool {
        matches!(self.exchange, Exchange::Sending { .. })
    }

    pub fn is_waiting_for(&self, job: u64) -> bool {
        matches!(self.exchange, Exchange::Sending { job: current, .. } if current == job)
    }

    pub fn release_display(&mut self) {
        if let Exchange::Completed(view) = &mut self.exchange {
            view.release_display();
        }
    }

    fn with_draft(
        id: TabId,
        binding: Option<ItemBinding>,
        draft: RequestDraft,
        unsaved: bool,
    ) -> Self {
        RequestTab {
            id,
            binding,
            draft,
            unsaved,
            edited_at: None,
            exchange: Exchange::Idle,
            editor_section: EditorSection::default(),
            response_section: ResponseSection::default(),
        }
    }
}
