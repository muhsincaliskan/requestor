use requestor_core::prepared::PreparedRequest;
use requestor_core::transport::SocketEvent;

use super::socket::SocketSession;
use super::tab::TabId;
use super::{Action, AppState, Effect};

impl AppState {
    pub(super) fn dispatch_socket_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::SocketEvent { tab, job, event } => self.socket_event(tab, job, event),
            Action::SocketEnded { tab, job, result } => self.socket_ended(tab, job, result),
            Action::SendSocketMessage(tab) => self.send_socket_message(tab, effects),
            other => self.dispatch_history_action(other, effects),
        }
    }

    pub(super) fn connect(
        &mut self,
        id: TabId,
        request: PreparedRequest,
        effects: &mut Vec<Effect>,
    ) {
        self.next_job += 1;
        let job = self.next_job;
        let replaced = self.tab_mut(id).and_then(|tab| {
            let session = SocketSession::connecting(job, tab.history_entry());
            tab.exchange.start_socket(session)
        });
        self.abandoned_jobs.extend(replaced);
        effects.push(Effect::OpenSocket {
            tab: id,
            job,
            request: Box::new(request),
            settings: Box::new(self.workspace.settings.request.clone()),
        });
    }

    pub(super) fn disconnect(&mut self, id: TabId) -> bool {
        let Some(session) = self.tab_mut(id).and_then(|tab| tab.exchange.live_socket()) else {
            return false;
        };
        let abandoned = session.disconnect();
        self.abandoned_jobs.extend(abandoned);
        true
    }

    pub(super) fn send_socket_message(&mut self, id: TabId, effects: &mut Vec<Effect>) {
        let Some(tab) = self.tab(id) else {
            return;
        };
        let Some(session) = tab.exchange.open_socket() else {
            return;
        };
        if !tab.message_draft.is_empty() {
            effects.push(Effect::SocketSend {
                job: session.job(),
                text: tab.message_draft.clone(),
            });
        }
    }

    fn socket_event(&mut self, id: TabId, job: u64, event: SocketEvent) {
        let entry = self
            .tab_mut(id)
            .and_then(|tab| tab.exchange.socket_mut(job))
            .and_then(|session| session.apply(event));
        self.remember_sent(entry);
    }

    fn socket_ended(&mut self, id: TabId, job: u64, result: Result<(), String>) {
        let entry = self
            .tab_mut(id)
            .and_then(|tab| tab.exchange.socket_mut(job))
            .and_then(|session| session.end(result));
        self.remember_sent(entry);
        self.abandoned_jobs.push(job);
    }
}
