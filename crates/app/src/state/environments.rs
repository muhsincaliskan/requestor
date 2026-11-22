use requestor_core::environment::Environment;
use requestor_core::workspace::{VariableAssignment, VariableHome};

use super::overlay::Overlay;
use super::{Action, AppState, Effect};

const NEW_ENVIRONMENT_NAME: &str = "New Environment";

impl AppState {
    pub(super) fn dispatch_environment_action(
        &mut self,
        action: Action,
        effects: &mut Vec<Effect>,
    ) {
        match action {
            Action::NewEnvironment => self.new_environment(),
            Action::SelectEnvironment(id) => self.select_environment(id),
            Action::EnvironmentEdited(id) => self.unsaved.touch_environment(&id, self.now),
            Action::AssignVariable(assignment) => self.assign_variable(&assignment),
            other => self.dispatch_transfer_action(other, effects),
        }
    }

    fn new_environment(&mut self) {
        let id = self
            .workspace
            .add_environment(Environment::new(NEW_ENVIRONMENT_NAME));
        self.unsaved.touch_environment(&id, self.now);
        self.select_environment(Some(id.clone()));
        self.overlay = Some(Overlay::EnvironmentEditor { environment_id: id });
    }

    fn assign_variable(&mut self, assignment: &VariableAssignment) {
        if !self.workspace.assign_variable(assignment) {
            return;
        }
        match &assignment.home {
            VariableHome::Environment { id, .. } => self.unsaved.touch_environment(id, self.now),
            VariableHome::Collection { id, .. }
            | VariableHome::Folder {
                collection_id: id, ..
            } => self.unsaved.touch_collection(id, self.now),
        }
    }

    fn select_environment(&mut self, id: Option<String>) {
        self.workspace.session.active_environment =
            id.filter(|id| self.workspace.environment(id).is_some());
        self.unsaved.touch_session(self.now);
    }
}
