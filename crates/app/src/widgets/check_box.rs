use eframe::egui::{CornerRadius, Response, Ui, WidgetText};

const BOX_RADIUS: u8 = 4;

pub fn check_box(ui: &mut Ui, checked: &mut bool, label: impl Into<WidgetText>) -> Response {
    ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        for state in [
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
            &mut widgets.open,
        ] {
            state.corner_radius = CornerRadius::same(BOX_RADIUS);
        }
        ui.checkbox(checked, label)
    })
    .inner
}
