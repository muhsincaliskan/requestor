use eframe::egui::{Label, RichText, Ui};
use requestor_core::workspace::DataFolder;

use crate::state::{Action, FolderPurpose};

const TITLE: &str = "Data folder";
const SYNC_HINT: &str = "Collections, environments, history and settings live here. Put this folder where Git, Google Drive or Dropbox syncs it to use the same data on other computers. Secret environment values and the proxy password stay in this computer's keychain; tokens typed into requests or collections, history and saved examples are synced as they are.";
const CHOOSE_LABEL: &str = "Choose folder…";
const DEFAULT_LABEL: &str = "Use the default folder";
const NEXT_PREFIX: &str = "After a restart:";
const UNKNOWN_TEXT: &str = "Requestor could not find its data folder on this computer.";
const COPY_HINT: &str = "An empty folder gets a copy of your current data. A folder that already has Requestor data is used as it is.";
const ROW_GAP: f32 = 8.0;

pub fn show(ui: &mut Ui, folder: Option<&DataFolder>, actions: &mut Vec<Action>) {
    ui.label(RichText::new(TITLE).strong());
    let Some(folder) = folder else {
        ui.weak(UNKNOWN_TEXT);
        return;
    };
    ui.add(
        Label::new(RichText::new(folder.location().display().to_string()).monospace())
            .selectable(true),
    );
    let next = folder.next_location();
    if next != folder.location() {
        ui.weak(format!("{NEXT_PREFIX} {}", next.display()));
    }
    ui.add_space(ROW_GAP);
    ui.horizontal(|ui| {
        if ui.button(CHOOSE_LABEL).on_hover_text(COPY_HINT).clicked() {
            actions.push(Action::ChooseFolder(FolderPurpose::DataFolder));
        }
        if !folder.is_default() && ui.button(DEFAULT_LABEL).clicked() {
            actions.push(Action::ResetDataFolder);
        }
    });
    ui.add_space(ROW_GAP);
    ui.label(RichText::new(SYNC_HINT).weak().small());
}
