mod tokens;

use eframe::egui::text::LayoutJob;
use eframe::egui::{Color32, FontId, TextFormat};
use requestor_core::json_tree::JsonKind;
use requestor_core::response::ResponseFormat;

use tokens::{TokenKind, Tokens};

pub struct SyntaxPalette {
    key: Color32,
    string: Color32,
    number: Color32,
    keyword: Color32,
    punctuation: Color32,
    tag: Color32,
    attribute: Color32,
    attribute_value: Color32,
    comment: Color32,
}

const DARK_KEY: Color32 = Color32::from_rgb(122, 162, 247);
const DARK_STRING: Color32 = Color32::from_rgb(158, 206, 106);
const DARK_NUMBER: Color32 = Color32::from_rgb(255, 158, 100);
const DARK_KEYWORD: Color32 = Color32::from_rgb(187, 154, 247);
const DARK_PUNCTUATION: Color32 = Color32::from_rgb(137, 146, 168);
const DARK_TAG: Color32 = Color32::from_rgb(247, 118, 142);
const DARK_ATTRIBUTE: Color32 = Color32::from_rgb(224, 175, 104);
const DARK_COMMENT: Color32 = Color32::from_rgb(105, 114, 140);

const LIGHT_KEY: Color32 = Color32::from_rgb(30, 80, 190);
const LIGHT_STRING: Color32 = Color32::from_rgb(34, 124, 45);
const LIGHT_NUMBER: Color32 = Color32::from_rgb(190, 90, 10);
const LIGHT_KEYWORD: Color32 = Color32::from_rgb(120, 60, 180);
const LIGHT_PUNCTUATION: Color32 = Color32::from_rgb(100, 108, 126);
const LIGHT_TAG: Color32 = Color32::from_rgb(190, 40, 70);
const LIGHT_ATTRIBUTE: Color32 = Color32::from_rgb(150, 100, 10);
const LIGHT_COMMENT: Color32 = Color32::from_rgb(130, 138, 156);

impl SyntaxPalette {
    pub const fn dark() -> Self {
        SyntaxPalette {
            key: DARK_KEY,
            string: DARK_STRING,
            number: DARK_NUMBER,
            keyword: DARK_KEYWORD,
            punctuation: DARK_PUNCTUATION,
            tag: DARK_TAG,
            attribute: DARK_ATTRIBUTE,
            attribute_value: DARK_STRING,
            comment: DARK_COMMENT,
        }
    }

    pub const fn light() -> Self {
        SyntaxPalette {
            key: LIGHT_KEY,
            string: LIGHT_STRING,
            number: LIGHT_NUMBER,
            keyword: LIGHT_KEYWORD,
            punctuation: LIGHT_PUNCTUATION,
            tag: LIGHT_TAG,
            attribute: LIGHT_ATTRIBUTE,
            attribute_value: LIGHT_STRING,
            comment: LIGHT_COMMENT,
        }
    }

    pub const fn for_mode(dark_mode: bool) -> Self {
        if dark_mode {
            Self::dark()
        } else {
            Self::light()
        }
    }

    pub fn key_color(&self) -> Color32 {
        self.key
    }

    pub fn json_color(&self, kind: JsonKind) -> Color32 {
        match kind {
            JsonKind::String => self.string,
            JsonKind::Number => self.number,
            JsonKind::Bool | JsonKind::Null => self.keyword,
            JsonKind::Array | JsonKind::Object => self.punctuation,
        }
    }

    fn color(&self, kind: TokenKind) -> Color32 {
        match kind {
            TokenKind::Key => self.key,
            TokenKind::String => self.string,
            TokenKind::Number => self.number,
            TokenKind::Keyword => self.keyword,
            TokenKind::Punctuation => self.punctuation,
            TokenKind::Tag => self.tag,
            TokenKind::Attribute => self.attribute,
            TokenKind::AttributeValue => self.attribute_value,
            TokenKind::Comment => self.comment,
            TokenKind::Text => Color32::PLACEHOLDER,
        }
    }
}

pub fn append_row(
    job: &mut LayoutJob,
    row: &str,
    format: ResponseFormat,
    palette: &SyntaxPalette,
    font: &FontId,
) {
    for (range, kind) in Tokens::new(row, format) {
        let section = TextFormat {
            font_id: font.clone(),
            color: palette.color(kind),
            ..TextFormat::default()
        };
        job.append(&row[range], 0.0, section);
    }
}

pub fn layout_row(
    row: &str,
    format: ResponseFormat,
    palette: &SyntaxPalette,
    font: &FontId,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    append_row(&mut job, row, format, palette, font);
    job
}

#[cfg(test)]
mod tests;
