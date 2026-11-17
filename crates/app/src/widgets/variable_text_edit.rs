mod completion;
mod popup;

use eframe::egui::{Context, Id, Key, Modifiers, Response, TextEdit, TextStyle, Ui};

use completion::step_selection;
use popup::{draw_popup, PopupState, Suggestion, POPUP_STATE_SALT};

pub struct VariableTextEdit<'a> {
    text: &'a mut String,
    names: &'a [String],
    hint: Option<&'a str>,
    width: Option<f32>,
    password: bool,
    rows: Option<usize>,
    id: Option<Id>,
}

impl<'a> VariableTextEdit<'a> {
    pub fn new(text: &'a mut String, names: &'a [String]) -> Self {
        VariableTextEdit {
            text,
            names,
            hint: None,
            width: None,
            password: false,
            rows: None,
            id: None,
        }
    }

    pub fn hint_text(mut self, hint: &'a str) -> Self {
        self.hint = Some(hint);
        self
    }

    pub fn desired_width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    pub fn multiline(mut self, rows: usize) -> Self {
        self.rows = Some(rows);
        self
    }

    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    pub fn show(mut self, ui: &mut Ui) -> Response {
        let id = self.id.unwrap_or_else(|| ui.next_auto_id());
        let ctx = ui.ctx().clone();
        let state_id = id.with(POPUP_STATE_SALT);
        let mut state: PopupState = ctx.data(|data| data.get_temp(state_id)).unwrap_or_default();
        let mut changed = self.handle_keys(ui, id, &mut state);
        let mut response = self.edit(id).show(ui).response.response;
        changed |= self.show_popup(&ctx, id, &response, &mut state);
        ctx.data_mut(|data| data.insert_temp(state_id, state));
        if changed {
            response.mark_changed();
        }
        response
    }

    fn edit(&mut self, id: Id) -> TextEdit<'_> {
        let mut edit = match self.rows {
            Some(rows) => TextEdit::multiline(&mut *self.text)
                .desired_rows(rows)
                .font(TextStyle::Monospace)
                .lock_focus(true),
            None => TextEdit::singleline(&mut *self.text),
        };
        edit = edit.id(id).password(self.password);
        if let Some(hint) = self.hint {
            edit = edit.hint_text(hint);
        }
        match self.width {
            Some(width) => edit.desired_width(width),
            None => edit,
        }
    }

    fn handle_keys(&mut self, ui: &Ui, id: Id, state: &mut PopupState) -> bool {
        let ctx = ui.ctx();
        let focused = ctx.memory(|memory| memory.has_focus(id));
        let suggestion = match focused && !self.names.is_empty() {
            true => Suggestion::find(ctx, id, self.text, self.names),
            false => None,
        };
        let Some(suggestion) = suggestion else {
            state.dismissed_start = None;
            return false;
        };
        if state.dismissed_start == Some(suggestion.partial.start()) {
            return false;
        }
        state.dismissed_start = None;
        state.selected = state.selected.min(suggestion.candidates.len() - 1);
        navigate(ctx, state, suggestion.candidates.len());
        if consume(ctx, Key::Escape) {
            state.dismissed_start = Some(suggestion.partial.start());
        }
        if consume(ctx, Key::Enter) || consume(ctx, Key::Tab) {
            suggestion.accept(ctx, id, self.text, state.selected);
            return true;
        }
        false
    }

    fn show_popup(
        &mut self,
        ctx: &Context,
        id: Id,
        response: &Response,
        state: &mut PopupState,
    ) -> bool {
        let shown = response.has_focus() || state.visible;
        let suggestion = Suggestion::find(ctx, id, self.text, self.names);
        let Some(suggestion) = suggestion
            .filter(|found| shown && state.dismissed_start != Some(found.partial.start()))
        else {
            state.visible = false;
            return false;
        };
        state.selected = state.selected.min(suggestion.candidates.len() - 1);
        let (clicked, hovered) = draw_popup(ctx, id, response, &suggestion, state);
        state.visible = response.has_focus() || hovered;
        state.moved = false;
        let Some(index) = clicked else {
            return false;
        };
        suggestion.accept(ctx, id, self.text, index);
        ctx.memory_mut(|memory| memory.request_focus(id));
        state.visible = false;
        true
    }
}

fn consume(ctx: &Context, key: Key) -> bool {
    ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key))
}

fn navigate(ctx: &Context, state: &mut PopupState, len: usize) {
    for (key, forward) in [(Key::ArrowDown, true), (Key::ArrowUp, false)] {
        if consume(ctx, key) {
            state.selected = step_selection(state.selected, forward, len);
            state.moved = true;
        }
    }
}
