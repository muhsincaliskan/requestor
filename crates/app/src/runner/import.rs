use std::thread;

use eframe::egui::Context;
use requestor_core::import::{import_address, ImportError, Imported};
use requestor_core::settings::RequestSettings;

use super::Runner;
use crate::state::Action;

const IMPORT_WORKER_NAME: &str = "requestor-import";

impl Runner {
    pub fn download_import(&mut self, address: String, settings: &RequestSettings, wake: &Context) {
        let transport = match self.transport_for(settings) {
            Ok(transport) => transport,
            Err(reason) => return self.report_import(Err(ImportError::Download(reason))),
        };
        let report = self.report.clone();
        let wake = wake.clone();
        let spawned = thread::Builder::new()
            .name(IMPORT_WORKER_NAME.to_string())
            .spawn(move || {
                let imported = import_address(&address, transport.as_ref());
                if report
                    .send(Action::ImportDownloaded(Box::new(imported)))
                    .is_ok()
                {
                    wake.request_repaint();
                }
            });
        if let Err(error) = spawned {
            self.report_import(Err(ImportError::Download(error.to_string())));
        }
    }

    fn report_import(&self, imported: Result<Imported, ImportError>) {
        self.report
            .send(Action::ImportDownloaded(Box::new(imported)))
            .ok();
    }
}
