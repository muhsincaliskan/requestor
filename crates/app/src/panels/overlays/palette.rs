use eframe::egui::{
    Align2, Context, Id, Key, Modal, Modifiers, RichText, TextEdit, Ui, Vec2, WidgetText,
};

use crate::palette::{Entry, Query};
use crate::state::{Action, AppState, PaletteForm};
use crate::widgets::{aligned_request_title, list_row, row_label};

const MODAL_ID: &str = "command-palette";
const SEARCH_HINT: &str = "Search commands, requests, environments and themes";
const NO_MATCH_TEXT: &str = "Nothing matches.";
const WIDTH: f32 = 560.0;
const MAX_RESULTS: usize = 12;
const TOP_OFFSET: Vec2 = Vec2::new(0.0, 96.0);

pub(super) fn show(
    ctx: &Context,
    state: &mut AppState,
    form: &mut PaletteForm,
    actions: &mut Vec<Action>,
) -> bool {
    let entries = Entry::all(state);
    let ranked = Query::new(&form.query).rank(&entries);
    let visible = &ranked[..ranked.len().min(MAX_RESULTS)];
    navigate(ctx, form, visible.len());
    let mut chosen = ctx
        .input(|input| input.key_pressed(Key::Enter))
        .then_some(form.selected);
    let id = Id::new(MODAL_ID);
    let area = Modal::default_area(id).anchor(Align2::CENTER_TOP, TOP_OFFSET);
    let shown = Modal::new(id).area(area).show(ctx, |ui| {
        ui.set_width(WIDTH);
        search_field(ui, form);
        if visible.is_empty() {
            ui.weak(NO_MATCH_TEXT);
        }
        for (index, entry) in visible.iter().enumerate() {
            if result_row(ui, entry, index == form.selected) {
                chosen = Some(index);
            }
        }
    });
    if let Some(entry) = chosen.and_then(|index| visible.get(index)) {
        entry.run(state, actions);
        return false;
    }
    !shown.should_close()
}

fn navigate(ctx: &Context, form: &mut PaletteForm, count: usize) {
    for (key, forward) in [(Key::ArrowDown, true), (Key::ArrowUp, false)] {
        if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key)) {
            form.move_selection(forward, count);
        }
    }
}

fn search_field(ui: &mut Ui, form: &mut PaletteForm) {
    let field = ui.add(
        TextEdit::singleline(&mut form.query)
            .hint_text(SEARCH_HINT)
            .desired_width(f32::INFINITY),
    );
    field.request_focus();
    if field.changed() {
        form.query_changed();
    }
}

fn result_row(ui: &mut Ui, entry: &Entry, selected: bool) -> bool {
    let detail = entry.shortcut().map_or_else(
        || entry.detail.clone(),
        |shortcut| shortcut.keys_text(ui.ctx()),
    );
    let title = match entry.method.as_deref() {
        Some(method) => WidgetText::from(aligned_request_title(ui, Some(method), &entry.title)),
        None => WidgetText::from(entry.title.as_str()),
    };
    let row = list_row(
        ui,
        selected,
        (row_label(title), RichText::new(detail).weak()),
    );
    let response = ui.add(row);
    if selected {
        response.scroll_to_me(None);
    }
    response.clicked()
}
