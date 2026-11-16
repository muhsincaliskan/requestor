use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

const JSON_EXTENSION: &str = "json";
const THEME_NAME: &str = "base16-ocean.dark";

pub struct JsonHighlighter {
    syntaxes: SyntaxSet,
    theme: Theme,
}

impl JsonHighlighter {
    pub fn new() -> Self {
        let theme = ThemeSet::load_defaults()
            .themes
            .remove(THEME_NAME)
            .expect("bundled theme exists");
        JsonHighlighter {
            syntaxes: SyntaxSet::load_defaults_newlines(),
            theme,
        }
    }

    pub fn layout_line(&self, line: &str, font_id: &FontId) -> LayoutJob {
        let mut job = LayoutJob::default();
        let mut highlighter = HighlightLines::new(self.json_syntax(), &self.theme);
        let terminated = format!("{line}\n");
        let Ok(ranges) = highlighter.highlight_line(&terminated, &self.syntaxes) else {
            job.append(line, 0.0, Self::plain_format(font_id));
            return job;
        };
        for (style, text) in ranges {
            let color =
                Color32::from_rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            let format = TextFormat {
                font_id: font_id.clone(),
                color,
                ..TextFormat::default()
            };
            job.append(text.trim_end_matches('\n'), 0.0, format);
        }
        job
    }

    fn plain_format(font_id: &FontId) -> TextFormat {
        TextFormat {
            font_id: font_id.clone(),
            ..TextFormat::default()
        }
    }

    fn json_syntax(&self) -> &SyntaxReference {
        self.syntaxes
            .find_syntax_by_extension(JSON_EXTENSION)
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text())
    }
}
