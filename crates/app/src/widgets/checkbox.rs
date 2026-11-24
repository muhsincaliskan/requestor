use std::sync::Arc;

use eframe::egui::{Checkbox, CornerRadius, Response, Ui};

const CHECKBOX_RADIUS: u8 = 4;

pub fn checkbox(ui: &mut Ui, checked: &mut bool, label: &str) -> Response {
    let outer = Arc::clone(ui.style());
    let radius = CornerRadius::same(CHECKBOX_RADIUS);
    let widgets = &mut ui.visuals_mut().widgets;
    for state in [
        &mut widgets.noninteractive,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        state.corner_radius = radius;
    }
    let response = if label.is_empty() {
        ui.add(Checkbox::without_text(checked))
    } else {
        ui.checkbox(checked, label)
    };
    ui.set_style(outer);
    response
}
