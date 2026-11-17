mod completion;
mod hints;
mod hover_preview;
mod placeholder_layout;
mod popup;

use eframe::egui::{
    Context, FontSelection, Id, Key, Margin, Modifiers, Response, TextBuffer, TextEdit, TextStyle,
    Ui,
};

use completion::step_selection;
pub use hints::VariableHints;
use placeholder_layout::PlaceholderLayout;
use popup::{draw_popup, PopupState, Suggestion, POPUP_STATE_SALT};

pub const FIELD_MARGIN: Margin = Margin::symmetric(6, 5);
const EDITOR_MARGIN: Margin = Margin::same(8);

pub struct VariableTextEdit<'a> {
    text: &'a mut String,
    hints: Option<&'a VariableHints>,
    hint: Option<&'a str>,
    width: Option<f32>,
    password: bool,
    rows: Option<usize>,
    id: Option<Id>,
}

impl<'a> VariableTextEdit<'a> {
    pub fn new(text: &'a mut String, hints: &'a VariableHints) -> Self {
        VariableTextEdit {
            hints: Some(hints),
            ..VariableTextEdit::plain(text)
        }
    }

    pub fn plain(text: &'a mut String) -> Self {
        VariableTextEdit {
            text,
            hints: None,
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
        let highlighted = self.hints.filter(|_| !self.password);
        let layout = PlaceholderLayout {
            names: highlighted.map_or(&[], VariableHints::names),
            font: FontSelection::from(self.text_style()).resolve(ui.style()),
            multiline: self.rows.is_some(),
        };
        let mut layouter = |ui: &Ui, text: &dyn TextBuffer, wrap_width: f32| {
            layout.galley(ui, text.as_str(), wrap_width)
        };
        let mut edit = self.edit(id);
        if highlighted.is_some() {
            edit = edit.layouter(&mut layouter);
        }
        let output = edit.show(ui);
        let mut response = match highlighted {
            Some(hints) => hover_preview::attach(hints, self.text, &output),
            None => output.response.response,
        };
        changed |= self.show_popup(&ctx, id, &response, &mut state);
        ctx.data_mut(|data| data.insert_temp(state_id, state));
        if changed {
            response.mark_changed();
        }
        response
    }

    fn suggestions(&self) -> &'a [String] {
        self.hints.map_or(&[], VariableHints::suggestions)
    }

    fn text_style(&self) -> TextStyle {
        match self.rows {
            Some(_) => TextStyle::Monospace,
            None => TextStyle::Body,
        }
    }

    fn edit(&mut self, id: Id) -> TextEdit<'_> {
        let style = self.text_style();
        let mut edit = match self.rows {
            Some(rows) => TextEdit::multiline(&mut *self.text)
                .desired_rows(rows)
                .margin(EDITOR_MARGIN)
                .lock_focus(true),
            None => TextEdit::singleline(&mut *self.text).margin(FIELD_MARGIN),
        };
        edit = edit.id(id).font(style).password(self.password);
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
        let names = self.suggestions();
        let suggestion = match focused && !names.is_empty() {
            true => Suggestion::find(ctx, id, self.text, names),
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
        let names = self.suggestions();
        if names.is_empty() {
            return false;
        }
        let shown = response.has_focus() || state.visible;
        let suggestion = Suggestion::find(ctx, id, self.text, names);
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
