use std::time::Instant;

use eframe::egui::{Context, Ui};
use requestor_core::workspace::WorkspaceStore;

use crate::panels;
use crate::runner::Runner;
use crate::saver::Saver;
use crate::shortcuts;
use crate::state::{Action, AppState, Effect};
use crate::theme::Look;

pub struct RequestorApp {
    state: AppState,
    runner: Runner,
    saver: Saver,
    started: Instant,
    look: Option<Look>,
    sends_active_tab_on_start: bool,
}

impl RequestorApp {
    pub fn new(store: WorkspaceStore) -> Self {
        let state = AppState::new(store.load(), store.clone());
        RequestorApp {
            state,
            runner: Runner::new(),
            saver: Saver::start(store),
            started: Instant::now(),
            look: None,
            sends_active_tab_on_start: false,
        }
    }

    pub fn sending_active_tab_on_start(mut self, enabled: bool) -> Self {
        self.sends_active_tab_on_start = enabled;
        self
    }

    fn now(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    fn apply(&mut self, actions: Vec<Action>, ctx: &Context) {
        let now = self.now();
        for action in actions {
            let effects = self.state.apply(action, now);
            self.execute(effects, ctx);
        }
    }

    fn execute(&mut self, effects: Vec<Effect>, ctx: &Context) {
        for effect in effects {
            match effect {
                Effect::Send {
                    tab,
                    job,
                    request,
                    settings,
                } => self.runner.send(tab, job, *request, &settings, ctx),
                Effect::CopyText(text) => ctx.copy_text(text),
                Effect::AbortSend(job) => self.runner.abort(job),
                storage => {
                    if let Err(reason) = self.saver.submit(storage) {
                        self.state.report_error(format!("Could not save: {reason}"));
                    }
                }
            }
        }
    }

    fn refresh_look(&mut self, ctx: &Context) {
        let wanted = Look::of(self.state.interface());
        if self.look != Some(wanted) {
            wanted.apply(ctx);
            self.look = Some(wanted);
        }
    }
}

impl eframe::App for RequestorApp {
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let mut pending: Vec<Action> = self.runner.finished().collect();
        if std::mem::take(&mut self.sends_active_tab_on_start) {
            pending.extend(self.state.active_tab.map(Action::Send));
        }
        self.apply(pending, ctx);
        let failures: Vec<String> = self.saver.failures().collect();
        for failure in failures {
            self.state.report_error(format!("Could not save {failure}"));
        }
        let now = self.now();
        let writes = self.state.tick(now);
        self.execute(writes, ctx);
        if let Some(wait) = self.state.seconds_until_next_save(now) {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
        }
        self.refresh_look(ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let mut actions = Vec::new();
        shortcuts::collect(&ctx, &mut self.state, &mut actions);
        let now = self.now();
        panels::show(ui, &mut self.state, now, &mut actions);
        self.apply(actions, &ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let writes = self.state.flush();
        for write in writes {
            if let Err(reason) = self.saver.submit(write) {
                eprintln!("could not save on exit: {reason}");
            }
        }
        self.saver.finish();
    }
}
