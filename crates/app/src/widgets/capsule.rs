use eframe::egui::{
    lerp, vec2, Align2, Rect, Response, Sense, StrokeKind, TextStyle, Ui, Widget, WidgetInfo,
    WidgetType,
};

const SEGMENT_PADDING: f32 = 12.0;
const KNOB_INSET: f32 = 2.0;

pub struct Capsule<'a, T> {
    value: &'a mut T,
    options: [(T, &'a str); 2],
}

impl<'a, T: Copy + PartialEq> Capsule<'a, T> {
    pub fn new(value: &'a mut T, options: [(T, &'a str); 2]) -> Self {
        Capsule { value, options }
    }
}

impl<T: Copy + PartialEq> Widget for Capsule<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let font = TextStyle::Button.resolve(ui.style());
        let segment = self
            .options
            .iter()
            .map(|(_, label)| {
                ui.fonts_mut(|fonts| {
                    fonts.layout_no_wrap(label.to_string(), font.clone(), ui.visuals().text_color())
                })
                .size()
                .x
            })
            .fold(0.0, f32::max)
            + SEGMENT_PADDING * 2.0;
        let height = ui.spacing().interact_size.y;
        let (rect, mut response) =
            ui.allocate_exact_size(vec2(segment * 2.0, height), Sense::click());
        let second = *self.value == self.options[1].0;
        if response.clicked() {
            let right = response
                .interact_pointer_pos()
                .is_some_and(|pointer| pointer.x > rect.center().x);
            let chosen = self.options[usize::from(right)].0;
            if chosen != *self.value {
                *self.value = chosen;
                response.mark_changed();
            }
        }
        let selected = self.options[usize::from(second)].1;
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::RadioGroup, ui.is_enabled(), true, selected)
        });
        if ui.is_rect_visible(rect) {
            paint(
                ui,
                &response,
                rect,
                segment,
                &self.options,
                *self.value == self.options[1].0,
            );
        }
        response
    }
}

fn paint<T>(
    ui: &Ui,
    response: &Response,
    rect: Rect,
    segment: f32,
    options: &[(T, &str); 2],
    second: bool,
) {
    let visuals = ui.style().interact(response);
    let radius = rect.height() / 2.0;
    let painter = ui.painter();
    painter.rect(
        rect,
        radius,
        ui.visuals().extreme_bg_color,
        visuals.bg_stroke,
        StrokeKind::Inside,
    );
    let progress = ui.ctx().animate_bool_responsive(response.id, second);
    let left = lerp(rect.left()..=(rect.left() + segment), progress);
    let knob = Rect::from_min_size(
        eframe::egui::pos2(left, rect.top()),
        vec2(segment, rect.height()),
    )
    .shrink(KNOB_INSET);
    painter.rect_filled(knob, knob.height() / 2.0, ui.visuals().selection.bg_fill);
    let font = TextStyle::Button.resolve(ui.style());
    for (index, (_, label)) in options.iter().enumerate() {
        let active = usize::from(second) == index;
        let color = match active {
            true => ui.visuals().selection.stroke.color,
            false => ui.visuals().weak_text_color(),
        };
        let center = rect.left_center() + vec2(segment * (index as f32 + 0.5), 0.0);
        painter.text(center, Align2::CENTER_CENTER, *label, font.clone(), color);
    }
}
