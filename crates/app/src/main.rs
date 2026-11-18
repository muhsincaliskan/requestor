mod app;
mod browser;
mod file_picker;
mod keychain;
mod palette;
mod panels;
mod response;
mod runner;
mod saver;
mod shortcuts;
mod state;
mod theme;
mod widgets;

use std::sync::Arc;

use app::RequestorApp;
use eframe::egui::ViewportBuilder;
use keychain::KeychainVaultStore;
use requestor_core::workspace::WorkspaceStore;

const APP_NAME: &str = "Requestor";
const FALLBACK_STORE_DIRECTORY: &str = ".requestor";
const SEND_ON_START_FLAG: &str = "--send-active-tab";
const INITIAL_WINDOW_SIZE: [f32; 2] = [1280.0, 800.0];
const MIN_WINDOW_SIZE: [f32; 2] = [720.0, 480.0];
#[cfg(not(target_os = "macos"))]
const WINDOW_ICON_RGBA: &[u8] = include_bytes!("../../../assets/icon/window-64.rgba");
#[cfg(not(target_os = "macos"))]
const WINDOW_ICON_SIZE: u32 = 64;

fn main() -> eframe::Result {
    let store = WorkspaceStore::in_default_location()
        .unwrap_or_else(|| WorkspaceStore::new(FALLBACK_STORE_DIRECTORY));
    let keychain = KeychainVaultStore::for_workspace(store.root());
    let store = store.with_vault(Arc::new(keychain));
    let send_on_start = std::env::args().any(|argument| argument == SEND_ON_START_FLAG);
    eframe::run_native(
        APP_NAME,
        eframe::NativeOptions {
            viewport: with_window_icon(
                ViewportBuilder::default()
                    .with_inner_size(INITIAL_WINDOW_SIZE)
                    .with_min_inner_size(MIN_WINDOW_SIZE),
            ),
            ..eframe::NativeOptions::default()
        },
        Box::new(move |creation_context| {
            theme::install_fonts(&creation_context.egui_ctx);
            Ok(Box::new(
                RequestorApp::new(store).sending_active_tab_on_start(send_on_start),
            ))
        }),
    )
}

#[cfg(not(target_os = "macos"))]
fn with_window_icon(viewport: ViewportBuilder) -> ViewportBuilder {
    viewport.with_icon(eframe::egui::IconData {
        rgba: WINDOW_ICON_RGBA.to_vec(),
        width: WINDOW_ICON_SIZE,
        height: WINDOW_ICON_SIZE,
    })
}

#[cfg(target_os = "macos")]
fn with_window_icon(viewport: ViewportBuilder) -> ViewportBuilder {
    viewport
}
