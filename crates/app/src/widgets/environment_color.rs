use eframe::egui::{Color32, Response, Sense, Ui, Vec2};
use requestor_core::environment::EnvironmentColor;

const SWATCH_DIAMETER: f32 = 10.0;

pub fn environment_color(ui: &Ui, color: EnvironmentColor) -> Color32 {
    let dark = ui.visuals().dark_mode;
    let (on_dark, on_light) = match color {
        EnvironmentColor::Gray => ((148, 156, 168), (110, 118, 129)),
        EnvironmentColor::Green => ((74, 190, 110), (22, 140, 64)),
        EnvironmentColor::Blue => ((96, 150, 245), (37, 99, 235)),
        EnvironmentColor::Purple => ((170, 130, 245), (124, 58, 237)),
        EnvironmentColor::Amber => ((240, 180, 60), (196, 128, 0)),
        EnvironmentColor::Red => ((240, 90, 90), (208, 32, 32)),
    };
    let (red, green, blue) = if dark { on_dark } else { on_light };
    Color32::from_rgb(red, green, blue)
}

pub fn environment_swatch(ui: &mut Ui, color: EnvironmentColor) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(SWATCH_DIAMETER), Sense::hover());
    let fill = environment_color(ui, color);
    ui.painter()
        .circle_filled(rect.center(), SWATCH_DIAMETER / 2.0, fill);
    response
}
