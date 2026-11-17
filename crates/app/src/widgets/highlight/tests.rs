use eframe::egui::FontId;
use requestor_core::response::ResponseFormat;

use super::tokens::{TokenKind, Tokens};
use super::{layout_row, SyntaxPalette};

const ODD_ROWS: [&str; 14] = [
    "",
    "\"",
    "\"unterminated",
    "\"escaped \\\" quote\" : 1",
    "\"ends with backslash\\",
    "{\"k\":\"v\",\"n\":-1.5e+10,\"t\":true,\"z\":null}",
    "héllo \"wörld\" 日本語 \u{1F600}",
    "<a href=\"x\" b='y'>text</a>",
    "<!-- open comment",
    "<div class=\"unterminated",
    "< lone",
    "</>",
    "<?xml version=\"1.0\"?>",
    "=\"'/>-\\",
];

const ALL_FORMATS: [ResponseFormat; 5] = [
    ResponseFormat::Json,
    ResponseFormat::Xml,
    ResponseFormat::Html,
    ResponseFormat::Text,
    ResponseFormat::Binary,
];

fn kinds(row: &str, format: ResponseFormat) -> Vec<(&str, TokenKind)> {
    Tokens::new(row, format)
        .map(|(range, kind)| (&row[range], kind))
        .collect()
}

#[test]
fn tokens_always_cover_the_row_without_gaps() {
    for format in ALL_FORMATS {
        for row in ODD_ROWS {
            let mut expected_start = 0;
            for (range, _) in Tokens::new(row, format) {
                assert_eq!(range.start, expected_start, "{row:?}");
                assert!(range.end > range.start, "{row:?}");
                assert!(row.is_char_boundary(range.end), "{row:?}");
                expected_start = range.end;
            }
            assert_eq!(expected_start, row.len(), "{row:?}");
        }
    }
}

#[test]
fn laid_out_sections_concatenate_to_the_row() {
    let font = FontId::monospace(12.0);
    for format in ALL_FORMATS {
        for row in ODD_ROWS {
            let job = layout_row(row, format, &SyntaxPalette::dark(), &font);
            assert_eq!(job.text, row);
            let joined: String = job
                .sections
                .iter()
                .map(|section| &job.text[section.byte_range.start.0..section.byte_range.end.0])
                .collect();
            assert_eq!(joined, row);
        }
    }
}

#[test]
fn json_keys_values_numbers_and_keywords_are_told_apart() {
    let tokens = kinds(
        "  \"name\" : \"v\", \"n\": -12.5e3, \"ok\": true",
        ResponseFormat::Json,
    );
    let find = |text: &str| {
        tokens
            .iter()
            .find(|(token, _)| *token == text)
            .map(|(_, kind)| *kind)
    };
    assert_eq!(find("\"name\""), Some(TokenKind::Key));
    assert_eq!(find("\"v\""), Some(TokenKind::String));
    assert_eq!(find("\"n\""), Some(TokenKind::Key));
    assert_eq!(find("-12.5e3"), Some(TokenKind::Number));
    assert_eq!(find("true"), Some(TokenKind::Keyword));
    assert_eq!(find(":"), Some(TokenKind::Punctuation));
}

#[test]
fn json_strings_keep_escaped_quotes_and_survive_cuts() {
    let escaped = kinds("\"a\\\"b\": 1", ResponseFormat::Json);
    assert_eq!(escaped[0], ("\"a\\\"b\"", TokenKind::Key));
    let cut = kinds("\"abc", ResponseFormat::Json);
    assert_eq!(cut, [("\"abc", TokenKind::String)]);
    let lone = kinds("\"", ResponseFormat::Json);
    assert_eq!(lone, [("\"", TokenKind::String)]);
    assert!(kinds("", ResponseFormat::Json).is_empty());
}

#[test]
fn markup_rows_split_tags_attributes_values_and_comments() {
    let tokens = kinds("<a href=\"x\">hi</a><!-- c -->", ResponseFormat::Html);
    assert_eq!(
        tokens,
        [
            ("<a", TokenKind::Tag),
            (" ", TokenKind::Text),
            ("href", TokenKind::Attribute),
            ("=", TokenKind::Punctuation),
            ("\"x\"", TokenKind::AttributeValue),
            (">", TokenKind::Tag),
            ("hi", TokenKind::Text),
            ("</a", TokenKind::Tag),
            (">", TokenKind::Tag),
            ("<!-- c -->", TokenKind::Comment),
        ]
    );
}

#[test]
fn self_closing_tags_and_unclosed_comments_end_cleanly() {
    let tokens = kinds("<br/>x", ResponseFormat::Xml);
    assert_eq!(tokens[1], ("/>", TokenKind::Tag));
    assert_eq!(tokens[2], ("x", TokenKind::Text));
    let comment = kinds("<!-- never closed", ResponseFormat::Xml);
    assert_eq!(comment, [("<!-- never closed", TokenKind::Comment)]);
}

#[test]
fn plain_text_is_a_single_section() {
    assert_eq!(
        kinds("{\"not\": json}", ResponseFormat::Text),
        [("{\"not\": json}", TokenKind::Text)]
    );
}
