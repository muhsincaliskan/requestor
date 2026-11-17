use serde_json::Value;

use super::literal;

const KEY_SEPARATOR: char = '.';
const INDEX_OPEN: char = '[';
const INDEX_CLOSE: char = ']';

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPath {
    segments: Vec<Segment>,
}

impl JsonPath {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let mut segments = Vec::new();
        if text.is_empty() {
            return Some(JsonPath { segments });
        }
        for part in text.split(KEY_SEPARATOR) {
            let index_start = part.find(INDEX_OPEN).unwrap_or(part.len());
            let key = &part[..index_start];
            if key.is_empty() && index_start == part.len() || !is_plain_key(key) {
                return None;
            }
            if !key.is_empty() {
                segments.push(Segment::Key(key.to_string()));
            }
            segments.extend(
                indices(&part[index_start..])?
                    .into_iter()
                    .map(Segment::Index),
            );
        }
        Some(JsonPath { segments })
    }

    pub fn from_accessor(accessor: &str) -> Option<Self> {
        let mut segments = Vec::new();
        let mut rest = accessor;
        while !rest.is_empty() {
            let (segment, remaining) = accessor_segment(rest)?;
            segments.push(segment);
            rest = remaining;
        }
        Some(JsonPath { segments })
    }

    pub fn text(&self) -> String {
        let mut text = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Key(key) if text.is_empty() => text.push_str(key),
                Segment::Key(key) => {
                    text.push(KEY_SEPARATOR);
                    text.push_str(key);
                }
                Segment::Index(index) => text.push_str(&format!("[{index}]")),
            }
        }
        text
    }

    pub fn accessor(&self) -> String {
        self.segments
            .iter()
            .map(|segment| match segment {
                Segment::Key(key) if is_identifier(key) => format!(".{key}"),
                Segment::Key(key) => format!("[{}]", literal::quote(key)),
                Segment::Index(index) => format!("[{index}]"),
            })
            .collect()
    }

    pub fn find<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        self.segments
            .iter()
            .try_fold(value, |current, segment| match segment {
                Segment::Key(key) => current.get(key),
                Segment::Index(index) => current.get(index),
            })
    }
}

fn indices(text: &str) -> Option<Vec<usize>> {
    let mut found = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let inner = rest.strip_prefix(INDEX_OPEN)?;
        let close = inner.find(INDEX_CLOSE)?;
        found.push(inner[..close].parse().ok()?);
        rest = &inner[close + 1..];
    }
    Some(found)
}

fn accessor_segment(text: &str) -> Option<(Segment, &str)> {
    if let Some(after) = text.strip_prefix(KEY_SEPARATOR) {
        let end = after
            .find([KEY_SEPARATOR, INDEX_OPEN])
            .unwrap_or(after.len());
        let key = &after[..end];
        return is_identifier(key).then(|| (Segment::Key(key.to_string()), &after[end..]));
    }
    let inner = text.strip_prefix(INDEX_OPEN)?;
    if let Some((key, after)) = literal::split_quoted(inner) {
        let rest = after.strip_prefix(INDEX_CLOSE)?;
        return is_plain_key(&key).then_some((Segment::Key(key), rest));
    }
    let close = inner.find(INDEX_CLOSE)?;
    let index = inner[..close].parse().ok()?;
    Some((Segment::Index(index), &inner[close + 1..]))
}

fn is_plain_key(key: &str) -> bool {
    !key.contains([KEY_SEPARATOR, INDEX_OPEN, INDEX_CLOSE]) && !key.contains(char::is_whitespace)
}

fn is_identifier(key: &str) -> bool {
    let mut characters = key.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || first == '$')
        && characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_' || rest == '$')
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::JsonPath;

    #[test]
    fn paths_find_keys_and_indices() {
        let body = json!({"data": {"items": [{"id": 7}], "content-type": "x"}});
        let path = JsonPath::parse("data.items[0].id").unwrap();
        assert_eq!(path.find(&body), Some(&json!(7)));
        assert_eq!(JsonPath::parse("").unwrap().find(&body), Some(&body));
        assert_eq!(JsonPath::parse("data.missing").unwrap().find(&body), None);
    }

    #[test]
    fn accessors_round_trip_through_the_path_text() {
        for text in ["data.items[0].id", "data.content-type", "[1][2]", "a"] {
            let path = JsonPath::parse(text).unwrap();
            assert_eq!(path.text(), text);
            assert_eq!(JsonPath::from_accessor(&path.accessor()), Some(path));
        }
        assert_eq!(
            JsonPath::parse("data.content-type").unwrap().accessor(),
            ".data[\"content-type\"]"
        );
    }

    #[test]
    fn malformed_paths_are_rejected() {
        for text in ["a..b", "a[x]", "a[1", "a b"] {
            assert_eq!(JsonPath::parse(text), None, "{text}");
        }
        assert_eq!(JsonPath::from_accessor(".a b"), None);
    }
}
