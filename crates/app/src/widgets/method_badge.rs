use eframe::egui::text::LayoutJob;
use eframe::egui::{Align, Color32, TextFormat, TextStyle, Ui};

const METHOD_GAP: &str = " ";
const METHOD_COLUMN_WIDTH: f32 = 42.0;
const METHOD_ABBREVIATIONS: [(&str, &str); 2] = [("DELETE", "DEL"), ("OPTIONS", "OPT")];

struct MethodColor {
    method: &'static str,
    on_dark: Color32,
    on_light: Color32,
}

const METHOD_COLORS: [MethodColor; 5] = [
    MethodColor::new(
        "GET",
        Color32::from_rgb(107, 203, 119),
        Color32::from_rgb(0, 127, 49),
    ),
    MethodColor::new(
        "POST",
        Color32::from_rgb(255, 196, 87),
        Color32::from_rgb(173, 122, 3),
    ),
    MethodColor::new(
        "PUT",
        Color32::from_rgb(116, 174, 246),
        Color32::from_rgb(9, 123, 237),
    ),
    MethodColor::new(
        "PATCH",
        Color32::from_rgb(192, 168, 225),
        Color32::from_rgb(98, 52, 151),
    ),
    MethodColor::new(
        "DELETE",
        Color32::from_rgb(247, 154, 142),
        Color32::from_rgb(180, 35, 24),
    ),
];

impl MethodColor {
    const fn new(method: &'static str, on_dark: Color32, on_light: Color32) -> Self {
        MethodColor {
            method,
            on_dark,
            on_light,
        }
    }

    fn pick(&self, dark_mode: bool) -> Color32 {
        if dark_mode {
            self.on_dark
        } else {
            self.on_light
        }
    }
}

pub fn method_color(ui: &Ui, method: &str) -> Color32 {
    METHOD_COLORS
        .iter()
        .find(|entry| entry.method.eq_ignore_ascii_case(method))
        .map_or(ui.visuals().weak_text_color(), |entry| {
            entry.pick(ui.visuals().dark_mode)
        })
}

pub fn request_title(ui: &Ui, method: Option<&str>, name: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    if let Some(method) = method {
        job.append(method, 0.0, badge_format(ui, method));
        job.append(METHOD_GAP, 0.0, TextFormat::default());
    }
    job.append(name, 0.0, title_format(ui));
    job
}

pub fn aligned_request_title(ui: &Ui, method: Option<&str>, name: &str) -> LayoutJob {
    let method = method.unwrap_or_default();
    let label = abbreviated(method);
    let badge = badge_format(ui, method);
    let label_width = ui.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(label.to_string(), badge.font_id.clone(), badge.color)
            .size()
            .x
    });
    let mut job = LayoutJob::default();
    job.append(label, 0.0, badge);
    job.append(
        name,
        (METHOD_COLUMN_WIDTH - label_width).max(0.0),
        title_format(ui),
    );
    job
}

fn badge_format(ui: &Ui, method: &str) -> TextFormat {
    TextFormat {
        font_id: TextStyle::Small.resolve(ui.style()),
        color: method_color(ui, method),
        valign: Align::Center,
        ..TextFormat::default()
    }
}

fn title_format(ui: &Ui) -> TextFormat {
    TextFormat {
        font_id: TextStyle::Body.resolve(ui.style()),
        color: ui.visuals().text_color(),
        ..TextFormat::default()
    }
}

fn abbreviated(method: &str) -> &str {
    METHOD_ABBREVIATIONS
        .iter()
        .find(|(full, _)| full.eq_ignore_ascii_case(method))
        .map_or(method, |(_, short)| short)
}

#[cfg(test)]
mod tests {
    use super::abbreviated;

    #[test]
    fn long_methods_are_abbreviated_and_others_kept() {
        assert_eq!(abbreviated("delete"), "DEL");
        assert_eq!(abbreviated("OPTIONS"), "OPT");
        assert_eq!(abbreviated("PATCH"), "PATCH");
    }
}
