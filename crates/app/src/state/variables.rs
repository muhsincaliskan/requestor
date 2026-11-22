use requestor_core::variables::VariableScope;
use requestor_core::workspace::VariableHomes;

use super::{AppState, RequestTab, TabId};

impl AppState {
    pub fn variable_scope(&self, tab: TabId) -> VariableScope {
        let binding = self.tab(tab).and_then(RequestTab::scope);
        self.workspace.scope_for(
            binding.map(|bound| bound.collection_id.as_str()),
            binding.map(|bound| bound.item_id.as_str()),
        )
    }

    pub fn variable_homes(&self, tab: TabId) -> VariableHomes {
        let binding = self.tab(tab).and_then(RequestTab::scope);
        self.workspace.variable_homes(
            binding.map(|bound| bound.collection_id.as_str()),
            binding.map(|bound| bound.item_id.as_str()),
        )
    }
}
