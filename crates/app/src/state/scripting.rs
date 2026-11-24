use requestor_core::response::HttpResponse;
use requestor_core::script::{Script, ScriptContext, ScriptReport};
use requestor_core::variables::{VariableLayer, VariableScope};
use requestor_core::workspace::ChangedStores;

use super::tab::TabId;
use super::AppState;

impl AppState {
    pub(super) fn run_before_script(&mut self, id: TabId) -> ScriptReport {
        let Some(tab) = self.tab(id) else {
            return ScriptReport::default();
        };
        let Some(script) = tab.draft.scripts.before.runnable() else {
            return ScriptReport::default();
        };
        let collection_id = tab.scope().map(|scope| scope.collection_id.clone());
        let report = self.run_script(id, &script, &[], None);
        self.store_extracted(&report, collection_id.as_deref());
        report
    }

    pub(super) fn run_after_script(
        &mut self,
        id: TabId,
        script: Option<&Script>,
        before: Option<ScriptReport>,
        response: &HttpResponse,
        collection_id: Option<&str>,
    ) -> Option<ScriptReport> {
        let Some(script) = script else {
            return before.filter(|report| !report.is_empty());
        };
        let mut report = before.unwrap_or_default();
        let after = self.run_script(id, script, &report.request_variables(), Some(response));
        self.store_extracted(&after, collection_id);
        report.absorb(after);
        Some(report)
    }

    fn run_script(
        &self,
        id: TabId,
        script: &Script,
        locals: &[(String, String)],
        response: Option<&HttpResponse>,
    ) -> ScriptReport {
        let scope = self
            .sendable(id)
            .map(|(_, scope)| with_locals(scope, locals))
            .unwrap_or_default();
        script.run(&ScriptContext {
            response,
            variables: &scope,
        })
    }

    fn store_extracted(&mut self, report: &ScriptReport, collection_id: Option<&str>) {
        let changed = self.workspace.store_extracted(report, collection_id);
        self.mark_extraction_changes(changed);
    }

    pub(super) fn mark_extraction_changes(&mut self, changed: ChangedStores) {
        if let Some(environment_id) = changed.environment_id {
            self.unsaved.touch_environment(&environment_id, self.now);
        }
        if let Some(collection_id) = changed.collection_id {
            self.unsaved.touch_collection(&collection_id, self.now);
        }
    }
}

pub(super) fn with_locals(scope: VariableScope, locals: &[(String, String)]) -> VariableScope {
    if locals.is_empty() {
        return scope;
    }
    let layer = locals
        .iter()
        .fold(VariableLayer::default(), |layer, (key, value)| {
            layer.with(key.as_str(), value.as_str())
        });
    scope.with_layer(layer)
}
