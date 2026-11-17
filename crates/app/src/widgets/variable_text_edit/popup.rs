use eframe::egui::text::{CCursor, CCursorRange};
use eframe::egui::{Area, Context, Frame, Id, Order, Response, ScrollArea, TextEdit, Ui};

use super::completion::{PartialVariable, MAX_SUGGESTIONS};

const POPUP_SALT: &str = "variable-suggestions";
pub(super) const POPUP_STATE_SALT: &str = "variable-suggestion-state";
const POPUP_LIST_SALT: &str = "variable-suggestion-list";
const POPUP_MAX_HEIGHT: f32 = 180.0;
const POPUP_MIN_WIDTH: f32 = 160.0;
const POPUP_MAX_WIDTH: f32 = 360.0;

#[derive(Clone, Default)]
pub(super) struct PopupState {
    pub(super) selected: usize,
    pub(super) dismissed_start: Option<usize>,
    pub(super) visible: bool,
    pub(super) moved: bool,
}

pub(super) struct Suggestion<'a> {
    pub(super) partial: PartialVariable,
    pub(super) cursor: usize,
    pub(super) candidates: Vec<&'a str>,
}

impl<'a> Suggestion<'a> {
    pub(super) fn find(ctx: &Context, id: Id, text: &str, names: &'a [String]) -> Option<Self> {
        let cursor: usize = TextEdit::load_state(ctx, id)?
            .cursor
            .char_range()?
            .primary
            .index
            .into();
        let partial = PartialVariable::at(text, cursor)?;
        let candidates = partial.candidates(names, MAX_SUGGESTIONS);
        (!candidates.is_empty()).then_some(Suggestion {
            partial,
            cursor,
            candidates,
        })
    }

    pub(super) fn accept(&self, ctx: &Context, id: Id, text: &mut String, index: usize) {
        let name = self.candidates[index.min(self.candidates.len() - 1)];
        let done = self.partial.complete(text, self.cursor, name);
        *text = done.text;
        place_cursor(ctx, id, done.cursor);
    }
}

pub(super) fn place_cursor(ctx: &Context, id: Id, char_index: usize) {
    let mut edit_state = TextEdit::load_state(ctx, id).unwrap_or_default();
    let range = CCursorRange::one(CCursor::new(char_index));
    edit_state.cursor.set_char_range(Some(range));
    TextEdit::store_state(ctx, id, edit_state);
}

pub(super) fn draw_popup(
    ctx: &Context,
    id: Id,
    field: &Response,
    suggestion: &Suggestion<'_>,
    state: &PopupState,
) -> (Option<usize>, bool) {
    let area = Area::new(id.with(POPUP_SALT))
        .order(Order::Foreground)
        .fixed_pos(field.rect.left_bottom())
        .constrain(true)
        .show(ctx, |ui| {
            Frame::popup(ui.style())
                .show(ui, |ui| {
                    ui.set_min_width(field.rect.width().clamp(POPUP_MIN_WIDTH, POPUP_MAX_WIDTH));
                    list_candidates(ui, id, suggestion, state)
                })
                .inner
        });
    let hovered = ctx
        .pointer_hover_pos()
        .is_some_and(|position| area.response.rect.contains(position));
    (area.inner, hovered)
}

fn list_candidates(
    ui: &mut Ui,
    id: Id,
    suggestion: &Suggestion<'_>,
    state: &PopupState,
) -> Option<usize> {
    let mut clicked = None;
    ScrollArea::vertical()
        .id_salt(id.with(POPUP_LIST_SALT))
        .max_height(POPUP_MAX_HEIGHT)
        .show(ui, |ui| {
            for (index, name) in suggestion.candidates.iter().enumerate() {
                let selected = index == state.selected;
                let row = ui.selectable_label(selected, *name);
                if selected && state.moved {
                    row.scroll_to_me(None);
                }
                if row.clicked() {
                    clicked = Some(index);
                }
            }
        });
    clicked
}
