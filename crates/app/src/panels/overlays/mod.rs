mod code;
mod collection_editor;
mod dialogs;
mod environment_editor;
mod import;
mod palette;
mod settings;
mod table;

use eframe::egui::{vec2, Align, Context, Frame, Id, Layout, Margin, Modal, Ui};

use crate::state::{Action, AppState, Overlay};

pub(super) const TITLE_GAP: f32 = 8.0;
const DIALOG_MIN_WIDTH: f32 = 320.0;
const DIALOG_MARGIN: i8 = 20;

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
        Overlay::CommandPalette(form) => palette::show(ctx, state, form, actions),
        Overlay::Code(form) => code::show(ctx, form, actions),
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
    let shown = modal(ctx, id).show(ctx, |ui| {
        ui.set_min_width(DIALOG_MIN_WIDTH);
        ui.heading(title);
        ui.add_space(TITLE_GAP);
        contents(ui)
    });
    shown.inner && !shown.should_close()
}

pub(super) fn modal(ctx: &Context, id: &str) -> Modal {
    Modal::new(Id::new(id))
        .frame(Frame::popup(&ctx.global_style()).inner_margin(Margin::same(DIALOG_MARGIN)))
}

pub(super) fn button_row<R>(ui: &mut Ui, contents: impl FnOnce(&mut Ui) -> R) -> R {
    ui.add_space(TITLE_GAP);
    let size = vec2(ui.min_rect().width(), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, Layout::right_to_left(Align::Center), contents)
        .inner
}
