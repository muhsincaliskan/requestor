use eframe::egui::{ComboBox, Context, Key, TextEdit, Ui};
use requestor_core::collection::Collection;

use super::{button_row, dialog};
use crate::state::{
    Action, AppState, DeleteConfirmation, ExportForm, ExportFormat, RenameForm, SaveTabForm, TabId,
};
use crate::widgets::action_button;

const CHOOSE_LOCATION_LABEL: &str = "Export to…";
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
const SECRET_VARIABLES_NOTE: &str = "Values of secret variables are always left out.";
const CREDENTIALS_FOUND: &str = "values look like passwords, tokens or keys (auth fields, headers or variables not marked secret). They are left out of the file.";
const KEEP_CREDENTIALS_LABEL: &str = "Keep them in the file";

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
        button_row(ui, |ui| {
            if action_button::primary(ui, SAVE_LABEL).clicked() {
                actions.push(Action::SaveAndCloseTab(tab));
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
        button_row(ui, |ui| {
            if action_button::destructive(ui, DELETE_LABEL).clicked() {
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
            return button_row(ui, |ui| {
                if action_button::primary(ui, NEW_COLLECTION_LABEL).clicked() {
                    actions.push(Action::NewCollectionForSave(form.clone()));
                    return false;
                }
                !ui.button(CANCEL_LABEL).clicked()
            });
        };
        let confirmed = name_field(ui, &mut form.name);
        collection_choice(ui, state, collection, form);
        folder_choice(ui, collection, form);
        button_row(ui, |ui| {
            let submitted = action_button::primary(ui, SAVE_LABEL).clicked() || confirmed;
            if let Some(submission) = form.submission().filter(|_| submitted) {
                actions.push(submission);
                return false;
            }
            !ui.button(CANCEL_LABEL).clicked()
        })
    })
}

fn name_field(ui: &mut Ui, name: &mut String) -> bool {
    let field = ui.add(TextEdit::singleline(name).desired_width(FIELD_WIDTH));
    let confirmed = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
    if !confirmed && ui.memory(|memory| memory.focused().is_none()) {
        field.request_focus();
    }
    confirmed
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
        button_row(ui, |ui| {
            if action_button::primary(ui, RENAME_LABEL).clicked() || confirmed {
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

pub fn export(
    ctx: &Context,
    form: &mut ExportForm,
    credentials: usize,
    actions: &mut Vec<Action>,
) -> bool {
    dialog(ctx, "export", "Export", |ui| {
        if form.offers_formats() {
            format_choice(ui, form);
        }
        credentials_choice(ui, form, credentials);
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut form.file_path)
                    .hint_text(EXPORT_PATH_HINT)
                    .desired_width(FIELD_WIDTH),
            );
            if ui.button(CHOOSE_LOCATION_LABEL).clicked() {
                actions.push(form.location_choice());
            }
        });
        if let Some(error) = &form.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        button_row(ui, |ui| {
            if action_button::primary(ui, EXPORT_LABEL).clicked() {
                match form.submission() {
                    Some(action) => actions.push(action),
                    None => form.error = Some(EXPORT_PATH_MISSING.to_string()),
                }
            }
            !ui.button(CANCEL_LABEL).clicked()
        })
    })
}

fn credentials_choice(ui: &mut Ui, form: &mut ExportForm, credentials: usize) {
    ui.weak(SECRET_VARIABLES_NOTE);
    if credentials == 0 {
        return;
    }
    let found = format!("{credentials} {CREDENTIALS_FOUND}");
    ui.colored_label(ui.visuals().warn_fg_color, found);
    ui.checkbox(&mut form.keep_credentials, KEEP_CREDENTIALS_LABEL);
}

fn format_choice(ui: &mut Ui, form: &mut ExportForm) {
    let current = form.format();
    ui.horizontal(|ui| {
        for format in ExportFormat::ALL {
            let clicked = ui
                .selectable_label(current == format, format.label())
                .on_hover_text(format.hint())
                .clicked();
            if clicked && current != format {
                form.set_format(format);
            }
        }
    });
}
