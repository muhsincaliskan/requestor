mod app;
mod panels;
mod response;
mod runner;
mod saver;
mod shortcuts;
mod state;
mod theme;
mod widgets;

use app::RequestorApp;
use requestor_core::workspace::WorkspaceStore;

const APP_NAME: &str = "Requestor";
const FALLBACK_STORE_DIRECTORY: &str = ".requestor";

fn main() -> eframe::Result {
    let store = WorkspaceStore::in_default_location()
        .unwrap_or_else(|| WorkspaceStore::new(FALLBACK_STORE_DIRECTORY));
    eframe::run_native(
        APP_NAME,
        eframe::NativeOptions::default(),
        Box::new(|_creation_context| Ok(Box::new(RequestorApp::new(store)))),
    )
}
