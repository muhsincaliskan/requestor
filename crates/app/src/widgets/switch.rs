use eframe::egui::{
    lerp, pos2, vec2, Response, Sense, StrokeKind, Ui, Widget, WidgetInfo, WidgetType,
};

const WIDTH_IN_HEIGHTS: f32 = 1.8;
const HEIGHT_SHARE: f32 = 0.8;
const KNOB_INSET: f32 = 0.75;

pub struct Switch<'a> {
    on: &'a mut bool,
}

impl<'a> Switch<'a> {
    pub fn new(on: &'a mut bool) -> Self {
        Switch { on }
    }
}

impl Widget for Switch<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let height = ui.spacing().interact_size.y * HEIGHT_SHARE;
        let size = vec2(height * WIDTH_IN_HEIGHTS, height);
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }
        let on = *self.on;
        response
            .widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, ui.is_enabled(), on, ""));
        if ui.is_rect_visible(rect) {
            let progress = ui.ctx().animate_bool_responsive(response.id, on);
            let visuals = ui.style().interact_selectable(&response, on);
            let track = if on {
                ui.visuals().hyperlink_color
            } else {
                visuals.bg_fill
            };
            let radius = rect.height() / 2.0;
            ui.painter()
                .rect(rect, radius, track, visuals.bg_stroke, StrokeKind::Inside);
            let x = lerp((rect.left() + radius)..=(rect.right() - radius), progress);
            ui.painter().circle_filled(
                pos2(x, rect.center().y),
                radius * KNOB_INSET,
                ui.visuals().window_fill,
            );
        }
        response
    }
}
