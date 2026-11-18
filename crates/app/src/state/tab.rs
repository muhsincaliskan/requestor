use requestor_core::collection::{Item, Request};
use requestor_core::draft::{RequestDraft, RulesDraft};
use requestor_core::history::HistoryEntry;
use requestor_core::settings::RequestOverrides;
use requestor_core::workspace::{SessionTab, Workspace};

use super::exchange::Exchange;
use super::schema_view::SchemaView;
use super::sections::{EditorSection, ResponseSection};

pub const UNTITLED_REQUEST: &str = "Untitled request";
const RESTORED_AT: f64 = 0.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TabId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemBinding {
    pub collection_id: String,
    pub item_id: String,
}

pub struct RequestTab {
    pub id: TabId,
    pub binding: Option<ItemBinding>,
    pub source: Option<ItemBinding>,
    pub draft: RequestDraft,
    pub unsaved: bool,
    pub edited_at: Option<f64>,
    pub exchange: Exchange,
    pub editor_section: EditorSection,
    pub response_section: ResponseSection,
    pub message_draft: String,
    pub schema: SchemaView,
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
        let draft = RequestDraft::from_request(item.display_name(), &request)
            .with_saved_rules(RulesDraft::from_events(item.event.as_ref()))
            .with_saved_overrides(RequestOverrides::from_extra(&item.extra));
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
        let stored_item = binding.as_ref().and_then(|binding| {
            workspace
                .collection(&binding.collection_id)?
                .find_item(&binding.item_id)
        });
        let stored = stored_item.and_then(Item::full_request);
        let saved_rules = RulesDraft::from_events(stored_item.and_then(|item| item.event.as_ref()));
        let saved_overrides = stored_item
            .map(|item| RequestOverrides::from_extra(&item.extra))
            .unwrap_or_default();
        let unsaved = saved.unsaved_request.is_some() || stored.is_none();
        let mut draft = match saved.unsaved_request.as_ref().or(stored.as_ref()) {
            Some(request) => RequestDraft::from_request(saved.name.as_str(), request),
            None => RequestDraft::blank(saved.name.as_str()),
        }
        .with_saved_rules(saved_rules)
        .with_saved_overrides(saved_overrides);
        if let Some(lines) = &saved.unsaved_tests {
            draft.rules = RulesDraft::from_script(lines);
        }
        if let Some(overrides) = saved.unsaved_overrides() {
            draft.overrides = overrides;
        }
        let mut tab = RequestTab::with_draft(id, binding, draft, unsaved);
        tab.edited_at = unsaved.then_some(RESTORED_AT);
        if tab.binding.is_none() {
            tab.source = saved
                .source()
                .map(|(collection, item)| ItemBinding::new(collection, item));
        }
        tab
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
            unsaved_tests: self
                .unsaved
                .then(|| self.draft.rules.script_lines())
                .flatten(),
            source_collection_id: self.source.as_ref().map(|from| from.collection_id.clone()),
            source_item_id: self.source.as_ref().map(|from| from.item_id.clone()),
            ..SessionTab::default()
        }
        .with_unsaved_overrides(self.unsaved.then_some(&self.draft.overrides))
    }

    pub fn scope(&self) -> Option<&ItemBinding> {
        self.binding.as_ref().or(self.source.as_ref())
    }

    pub fn history_entry(&self) -> HistoryEntry {
        let entry = HistoryEntry::new(self.draft.name.as_str(), self.draft.to_request());
        match self.scope() {
            Some(scope) => entry.sent_from(&scope.collection_id, &scope.item_id),
            None => entry,
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

    pub fn awaits_manual_save(&self, autosave: bool) -> bool {
        self.unsaved && !(autosave && self.binding.is_some())
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

    pub fn is_connected(&self) -> bool {
        matches!(&self.exchange, Exchange::Socket(session) if session.is_live())
    }

    pub fn abandon_send(&mut self) -> Option<u64> {
        let job = match &self.exchange {
            Exchange::Sending { job, .. } => *job,
            Exchange::Socket(session) => session.live_job()?,
            _ => return None,
        };
        self.exchange = Exchange::Idle;
        Some(job)
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
            source: None,
            draft,
            unsaved,
            edited_at: None,
            exchange: Exchange::Idle,
            editor_section: EditorSection::default(),
            response_section: ResponseSection::default(),
            message_draft: String::new(),
            schema: SchemaView::default(),
        }
    }
}
