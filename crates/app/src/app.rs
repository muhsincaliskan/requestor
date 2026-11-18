use std::time::Instant;

use eframe::egui::{Context, Ui};
use requestor_core::workspace::{DataFolder, WorkspaceStore};

use crate::browser;
use crate::file_picker;
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
    update_check_pending: bool,
}

impl RequestorApp {
    pub fn new(
        mut store: WorkspaceStore,
        data_folder: Option<(DataFolder, std::io::Result<()>)>,
    ) -> Self {
        let loaded = store.load();
        let mut state = AppState::new(loaded, store);
        if let Some((folder, settled)) = data_folder {
            state = state.with_data_folder(folder, settled);
        }
        RequestorApp {
            state,
            runner: Runner::new(),
            saver: Saver::start(),
            started: Instant::now(),
            look: None,
            sends_active_tab_on_start: false,
            update_check_pending: true,
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
        let mut queue = std::collections::VecDeque::from(actions);
        while let Some(action) = queue.pop_front() {
            let effects = self.state.apply(action, now);
            queue.extend(self.execute(effects, ctx));
        }
    }

    fn execute(&mut self, effects: Vec<Effect>, ctx: &Context) -> Vec<Action> {
        let mut follow_ups = Vec::new();
        for effect in effects {
            match effect {
                Effect::Send {
                    tab,
                    job,
                    request,
                    settings,
                } => self.runner.send(tab, job, *request, &settings, ctx),
                Effect::OpenSocket {
                    tab,
                    job,
                    request,
                    settings,
                } => self.runner.open_socket(tab, job, *request, &settings, ctx),
                Effect::FetchSchema {
                    tab,
                    job,
                    request,
                    settings,
                } => self.runner.fetch_schema(tab, job, *request, &settings, ctx),
                Effect::SocketSend { job, text } => self.runner.socket_send(job, text),
                Effect::CopyText(text) => ctx.copy_text(text),
                Effect::OpenUrl(url) => {
                    if let Err(error) = browser::open(&url) {
                        self.state
                            .report_error(format!("Could not open the browser: {error}"));
                    }
                }
                Effect::AbortSend(job) => self.runner.abort(job),
                Effect::DownloadImport {
                    job,
                    address,
                    settings,
                } => self.runner.download_import(job, address, &settings, ctx),
                Effect::PickImportFile => {
                    follow_ups.extend(file_picker::import_file().map(Action::ImportFile));
                }
                Effect::PickExportLocation {
                    target,
                    suggested,
                    keep_credentials,
                } => follow_ups.extend(file_picker::export_location(&suggested).map(|file_path| {
                    Action::Export {
                        target,
                        file_path,
                        keep_credentials,
                    }
                })),
                Effect::PickCertificateFile(file) => follow_ups.extend(
                    file_picker::certificate_file()
                        .map(|path| Action::CertificateFilePicked { file, path }),
                ),
                Effect::PickFolder(purpose) => follow_ups.extend(
                    file_picker::folder(&purpose)
                        .map(|path| Action::FolderPicked { purpose, path }),
                ),
                storage => {
                    if let Err(reason) = self.saver.submit(storage) {
                        self.state.report_error(format!("Could not save: {reason}"));
                    }
                }
            }
        }
        follow_ups
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
        let mut pending = self.runner.finished();
        if std::mem::take(&mut self.sends_active_tab_on_start) {
            pending.extend(self.state.active_tab.map(Action::Send));
        }
        if std::mem::take(&mut self.update_check_pending)
            && self.state.interface().check_for_updates
        {
            self.runner
                .check_for_update(&self.state.workspace.settings.request, ctx);
        }
        self.apply(pending, ctx);
        let failures: Vec<String> = self.saver.failures().collect();
        for failure in failures {
            self.state.report_error(format!("Could not save {failure}"));
        }
        let now = self.now();
        let writes = self.state.tick(now);
        let follow_ups = self.execute(writes, ctx);
        self.apply(follow_ups, ctx);
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
