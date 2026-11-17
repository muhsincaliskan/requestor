mod app;
mod file_picker;
mod panels;
mod response;
mod runner;
mod saver;
mod shortcuts;
mod state;
mod theme;
mod widgets;

use app::RequestorApp;
use eframe::egui::ViewportBuilder;
use requestor_core::workspace::WorkspaceStore;

const APP_NAME: &str = "Requestor";
const FALLBACK_STORE_DIRECTORY: &str = ".requestor";
const SEND_ON_START_FLAG: &str = "--send-active-tab";
const INITIAL_WINDOW_SIZE: [f32; 2] = [1280.0, 800.0];
const MIN_WINDOW_SIZE: [f32; 2] = [720.0, 480.0];

fn main() -> eframe::Result {
    let store = WorkspaceStore::in_default_location()
        .unwrap_or_else(|| WorkspaceStore::new(FALLBACK_STORE_DIRECTORY));
    let send_on_start = std::env::args().any(|argument| argument == SEND_ON_START_FLAG);
    eframe::run_native(
        APP_NAME,
        eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_inner_size(INITIAL_WINDOW_SIZE)
                .with_min_inner_size(MIN_WINDOW_SIZE),
            ..eframe::NativeOptions::default()
        },
        Box::new(move |_creation_context| {
            Ok(Box::new(
                RequestorApp::new(store).sending_active_tab_on_start(send_on_start),
            ))
        }),
    )
}
