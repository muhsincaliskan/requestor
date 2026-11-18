use eframe::egui::Context;

use super::dialog;
use super::shared_fields::{auth_section, headers_section};
use super::table::{footer_done, name_field};
use crate::state::{Action, AppState};
use crate::widgets::variable_text_edit::VariableHints;

const DIALOG_ID: &str = "folder-editor";
const TITLE: &str = "Edit folder";
const SECTION_GAP: f32 = 10.0;

pub fn show(
    ctx: &Context,
    state: &mut AppState,
    (collection_id, folder_id): (&str, &str),
    actions: &mut Vec<Action>,
) -> bool {
    let hints = VariableHints::new(
        state
            .workspace
            .scope_for(Some(collection_id), Some(folder_id)),
        state.interface().variable_autocomplete,
    );
    let Some(folder) = state
        .workspace
        .collection_mut(collection_id)
        .and_then(|collection| collection.find_item_mut(folder_id))
    else {
        return false;
    };
    dialog(ctx, DIALOG_ID, TITLE, |ui| {
        let mut name = folder.display_name().to_string();
        let mut changed = name_field(ui, &mut name);
        if changed {
            folder.name = Some(name);
        }
        ui.add_space(SECTION_GAP);
        changed |= auth_section(ui, &mut folder.auth, &hints);
        ui.add_space(SECTION_GAP);
        let mut headers = folder.shared_headers();
        if headers_section(ui, &mut headers) {
            folder.set_shared_headers(&headers);
            changed = true;
        }
        if changed {
            actions.push(Action::CollectionEdited(collection_id.to_string()));
        }
        !footer_done(ui)
    })
}
