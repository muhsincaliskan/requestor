use eframe::egui::{Align, Layout, RichText, Ui};

use crate::widgets::switch::Switch;

const LABEL_SHARE: f32 = 0.62;
const SCROLLBAR_GUTTER: f32 = 14.0;

pub struct Toggle<T> {
    pub label: &'static str,
    pub description: &'static str,
    pub field: fn(&mut T) -> &mut bool,
}

pub fn row(
    ui: &mut Ui,
    label: &str,
    description: &str,
    control: impl FnOnce(&mut Ui) -> bool,
) -> bool {
    let label_width = ui.available_width() * LABEL_SHARE;
    let changed = ui
        .horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width(label_width);
                ui.label(label);
                if !description.is_empty() {
                    ui.label(RichText::new(description).weak().small());
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(SCROLLBAR_GUTTER);
                control(ui)
            })
            .inner
        })
        .inner;
    ui.separator();
    changed
}

pub fn toggles<T>(ui: &mut Ui, table: &[Toggle<T>], target: &mut T) -> bool {
    let mut changed = false;
    for toggle in table {
        changed |= row(ui, toggle.label, toggle.description, |ui| {
            ui.add(Switch::new((toggle.field)(target))).changed()
        });
    }
    changed
}
