mod collection_editor;
mod dialogs;
mod environment_editor;
mod import;
mod settings;
mod table;

use eframe::egui::{Context, Id, Modal, Ui};

use crate::state::{Action, AppState, Overlay};

pub(super) const TITLE_GAP: f32 = 8.0;

pub fn show(ctx: &Context, state: &mut AppState, actions: &mut Vec<Action>) {
    let Some(mut overlay) = state.overlay.take() else {
        return;
    };
    let stays_open = match &mut overlay {
        Overlay::Settings => settings::show(ctx, state, actions),
        Overlay::Import(form) => import::show(ctx, form, actions),
        Overlay::EnvironmentEditor { environment_id } => {
            environment_editor::show(ctx, state, environment_id, actions)
        }
        Overlay::CollectionEditor { collection_id } => {
            collection_editor::show(ctx, state, collection_id, actions)
        }
        Overlay::ConfirmCloseTab(tab) => dialogs::confirm_close_tab(ctx, state, *tab, actions),
        Overlay::ConfirmDelete(confirmation) => dialogs::confirm_delete(ctx, confirmation, actions),
        Overlay::SaveTab(form) => dialogs::save_tab(ctx, state, form, actions),
        Overlay::Rename(form) => dialogs::rename(ctx, form, actions),
        Overlay::Export(form) => dialogs::export(ctx, form, actions),
    };
    if stays_open {
        state.overlay = Some(overlay);
    }
}

pub(super) fn dialog(
    ctx: &Context,
    id: &str,
    title: &str,
    contents: impl FnOnce(&mut Ui) -> bool,
) -> bool {
    let shown = Modal::new(Id::new(id)).show(ctx, |ui| {
        ui.heading(title);
        ui.add_space(TITLE_GAP);
        contents(ui)
    });
    shown.inner && !shown.should_close()
}
