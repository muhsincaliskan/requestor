use requestor_core::environment::Environment;

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
            other => self.dispatch_transfer_action(other, effects),
        }
    }

    fn new_environment(&mut self) {
        let id = self
            .workspace
            .add_environment(Environment::new(NEW_ENVIRONMENT_NAME));
        self.unsaved.touch_environment(&id, self.now);
        self.overlay = Some(Overlay::EnvironmentEditor { environment_id: id });
    }

    fn select_environment(&mut self, id: Option<String>) {
        self.workspace.session.active_environment =
            id.filter(|id| self.workspace.environment(id).is_some());
        self.unsaved.touch_session(self.now);
    }
}
