use eframe::egui::{vec2, Align2, Atom, AtomExt, Button, IntoAtoms, Ui, WidgetText};

pub fn list_row<'a>(ui: &Ui, selected: bool, atoms: impl IntoAtoms<'a>) -> Button<'a> {
    Button::selectable(selected, atoms)
        .truncate()
        .min_size(vec2(ui.available_width(), 0.0))
}

pub fn row_label<'a>(text: impl Into<WidgetText>) -> Atom<'a> {
    text.into()
        .atom_grow(true)
        .atom_shrink(true)
        .atom_align(Align2::LEFT_CENTER)
}
