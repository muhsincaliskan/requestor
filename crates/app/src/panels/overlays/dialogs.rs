use std::path::PathBuf;

use eframe::egui::{ComboBox, Context, Key, TextEdit, Ui};
use requestor_core::collection::Collection;

use super::dialog;
use crate::state::{
    Action, AppState, DeleteConfirmation, ExportForm, RenameForm, SaveTabForm, TabId,
};

const CANCEL_LABEL: &str = "Cancel";
const SAVE_LABEL: &str = "Save";
const DISCARD_LABEL: &str = "Discard changes";
const DELETE_LABEL: &str = "Delete";
const RENAME_LABEL: &str = "Rename";
const EXPORT_LABEL: &str = "Export";
const NEW_COLLECTION_LABEL: &str = "New collection";
const NO_COLLECTION_TEXT: &str = "Create a collection to save this request into.";
const COLLECTION_ROOT_LABEL: &str = "Collection root";
const FOLDER_PATH_SEPARATOR: &str = " / ";
const EXPORT_PATH_HINT: &str = "File to write";
const EXPORT_PATH_MISSING: &str = "Enter the file to write";
const FIELD_WIDTH: f32 = 320.0;

pub fn confirm_close_tab(
    ctx: &Context,
    state: &AppState,
    tab: TabId,
    actions: &mut Vec<Action>,
) -> bool {
    let Some(open_tab) = state.tab(tab) else {
        return false;
    };
    let title = format!("Close \"{}\"?", open_tab.draft.name);
    dialog(ctx, "confirm-close-tab", &title, |ui| {
        ui.label("This request has changes that are not saved.");
        buttons(ui, |ui| {
            if ui.button(SAVE_LABEL).clicked() {
                actions.push(Action::SaveTab(tab));
                return false;
            }
            if ui.button(DISCARD_LABEL).clicked() {
                actions.push(Action::ForceCloseTab(tab));
                return false;
            }
            !ui.button(CANCEL_LABEL).clicked()
        })
    })
}

pub fn confirm_delete(
    ctx: &Context,
    confirmation: &DeleteConfirmation,
    actions: &mut Vec<Action>,
) -> bool {
    let title = format!("Delete \"{}\"?", confirmation.name);
    dialog(ctx, "confirm-delete", &title, |ui| {
        ui.label("This cannot be undone.");
        buttons(ui, |ui| {
            if ui.button(DELETE_LABEL).clicked() {
                actions.push(Action::Delete(confirmation.target.clone()));
                return false;
            }
            !ui.button(CANCEL_LABEL).clicked()
        })
    })
}

pub fn save_tab(
    ctx: &Context,
    state: &AppState,
    form: &mut SaveTabForm,
    actions: &mut Vec<Action>,
) -> bool {
    dialog(ctx, "save-tab", "Save request", |ui| {
        let chosen = form
            .collection_id
            .as_deref()
            .and_then(|id| state.workspace.collection(id));
        let Some(collection) = chosen else {
            ui.label(NO_COLLECTION_TEXT);
            return buttons(ui, |ui| {
                if ui.button(NEW_COLLECTION_LABEL).clicked() {
                    actions.push(Action::NewCollection);
                    return false;
                }
                !ui.button(CANCEL_LABEL).clicked()
            });
        };
        ui.add(TextEdit::singleline(&mut form.name).desired_width(FIELD_WIDTH));
        collection_choice(ui, state, collection, form);
        folder_choice(ui, collection, form);
        buttons(ui, |ui| {
            if ui.button(SAVE_LABEL).clicked() {
                actions.extend(form.submission());
                return false;
            }
            !ui.button(CANCEL_LABEL).clicked()
        })
    })
}

fn collection_choice(ui: &mut Ui, state: &AppState, chosen: &Collection, form: &mut SaveTabForm) {
    ComboBox::from_id_salt("save-tab-collection")
        .width(FIELD_WIDTH)
        .selected_text(&chosen.info.name)
        .show_ui(ui, |ui| {
            for collection in &state.workspace.collections {
                let selected = collection.id() == chosen.id();
                if ui
                    .selectable_label(selected, &collection.info.name)
                    .clicked()
                    && !selected
                {
                    form.collection_id = Some(collection.id().to_string());
                    form.folder_id = None;
                }
            }
        });
}

fn folder_choice(ui: &mut Ui, collection: &Collection, form: &mut SaveTabForm) {
    let folders = collection.folder_paths(FOLDER_PATH_SEPARATOR);
    let selected = folders
        .iter()
        .find(|(id, _)| Some(id) == form.folder_id.as_ref())
        .map_or(COLLECTION_ROOT_LABEL, |(_, path)| path.as_str());
    ComboBox::from_id_salt("save-tab-folder")
        .width(FIELD_WIDTH)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut form.folder_id, None, COLLECTION_ROOT_LABEL);
            for (id, path) in &folders {
                ui.selectable_value(&mut form.folder_id, Some(id.clone()), path);
            }
        });
}

pub fn rename(ctx: &Context, form: &mut RenameForm, actions: &mut Vec<Action>) -> bool {
    dialog(ctx, "rename", "Rename", |ui| {
        let field = ui.add(TextEdit::singleline(&mut form.name).desired_width(FIELD_WIDTH));
        let confirmed = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
        if !field.has_focus() && !confirmed {
            field.request_focus();
        }
        buttons(ui, |ui| {
            if ui.button(RENAME_LABEL).clicked() || confirmed {
                actions.push(Action::Rename {
                    target: form.target.clone(),
                    name: form.name.clone(),
                });
                return false;
            }
            !ui.button(CANCEL_LABEL).clicked()
        })
    })
}

pub fn export(ctx: &Context, form: &mut ExportForm, actions: &mut Vec<Action>) -> bool {
    dialog(ctx, "export", "Export", |ui| {
        ui.add(
            TextEdit::singleline(&mut form.file_path)
                .hint_text(EXPORT_PATH_HINT)
                .desired_width(FIELD_WIDTH),
        );
        if let Some(error) = &form.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        buttons(ui, |ui| {
            if ui.button(EXPORT_LABEL).clicked() {
                if form.file_path.trim().is_empty() {
                    form.error = Some(EXPORT_PATH_MISSING.to_string());
                    return true;
                }
                actions.push(Action::Export {
                    target: form.target.clone(),
                    file_path: PathBuf::from(form.file_path.trim()),
                });
            }
            !ui.button(CANCEL_LABEL).clicked()
        })
    })
}

fn buttons(ui: &mut Ui, contents: impl FnOnce(&mut Ui) -> bool) -> bool {
    ui.add_space(super::TITLE_GAP);
    ui.horizontal(contents).inner
}
