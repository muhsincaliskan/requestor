use std::sync::Arc;

use eframe::egui::text::{LayoutJob, TextWrapping};
use eframe::egui::{Color32, FontId, Galley, TextFormat, Ui};
use requestor_core::variables::{TextSegment, TextSegments};

pub struct PlaceholderLayout<'a> {
    pub names: &'a [String],
    pub font: FontId,
    pub multiline: bool,
}

impl PlaceholderLayout<'_> {
    pub fn galley(&self, ui: &Ui, text: &str, wrap_width: f32) -> Arc<Galley> {
        let mut job = LayoutJob {
            break_on_newline: self.multiline,
            ..Default::default()
        };
        if self.multiline {
            job.wrap = TextWrapping {
                max_width: wrap_width,
                ..Default::default()
            };
        }
        for segment in TextSegments::new(text) {
            let format = TextFormat::simple(self.font.clone(), self.color(ui, &segment));
            job.append(segment.text(), 0.0, format);
        }
        ui.fonts_mut(|fonts| fonts.layout_job(job))
    }

    fn color(&self, ui: &Ui, segment: &TextSegment<'_>) -> Color32 {
        let visuals = ui.visuals();
        match segment {
            TextSegment::Plain(_) => visuals
                .override_text_color
                .unwrap_or_else(|| visuals.widgets.inactive.text_color()),
            _ if segment.is_known_in(self.names) => visuals.hyperlink_color,
            _ => visuals.error_fg_color,
        }
    }
}
