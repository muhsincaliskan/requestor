use super::url::{Url, SCHEME_SEPARATOR};

const PATH_VARIABLE_MARK: char = ':';
const PATH_SEPARATOR: char = '/';
const PATH_TERMINATORS: [char; 2] = ['?', '#'];

pub fn path_variable_names(url: &str) -> Vec<&str> {
    let (start, end) = path_span(url);
    url[start..end]
        .split(PATH_SEPARATOR)
        .filter_map(|segment| segment.strip_prefix(PATH_VARIABLE_MARK))
        .filter(|name| !name.is_empty())
        .collect()
}

impl Url {
    pub fn to_sendable_text(&self) -> String {
        let text = self.to_text();
        let Url::Parts(parts) = self else {
            return text;
        };
        let values: Vec<(&str, String)> = parts
            .variable
            .iter()
            .flatten()
            .filter(|variable| variable.is_enabled())
            .filter_map(|variable| Some((variable.key.as_deref()?, variable.text_value()?)))
            .collect();
        if values.is_empty() {
            return text;
        }
        let (start, end) = path_span(&text);
        let path: Vec<&str> = text[start..end]
            .split(PATH_SEPARATOR)
            .map(|segment| substituted(segment, &values))
            .collect();
        format!(
            "{}{}{}",
            &text[..start],
            path.join(&PATH_SEPARATOR.to_string()),
            &text[end..]
        )
    }
}

fn substituted<'a>(segment: &'a str, values: &'a [(&str, String)]) -> &'a str {
    segment
        .strip_prefix(PATH_VARIABLE_MARK)
        .and_then(|name| values.iter().find(|(key, _)| *key == name))
        .map_or(segment, |(_, value)| value.as_str())
}

fn path_span(url: &str) -> (usize, usize) {
    let end = url.find(PATH_TERMINATORS).unwrap_or(url.len());
    let head = &url[..end];
    let authority_start = head
        .find(SCHEME_SEPARATOR)
        .map_or(0, |position| position + SCHEME_SEPARATOR.len());
    let start = head[authority_start..]
        .find(PATH_SEPARATOR)
        .map_or(end, |position| authority_start + position);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::path_variable_names;
    use crate::collection::{Url, UrlParts, Variable};

    #[test]
    fn names_come_from_path_segments_only() {
        assert_eq!(
            path_variable_names("https://x.test:8080/users/:id/posts/:post?next=/:nope#/:frag"),
            ["id", "post"]
        );
        assert_eq!(path_variable_names("{{baseUrl}}/pets/:petId"), ["petId"]);
        assert!(path_variable_names("localhost:3000").is_empty());
        assert!(path_variable_names("https://x.test/a/:/b").is_empty());
    }

    #[test]
    fn sendable_text_replaces_enabled_variables_that_have_values() {
        let mut parts = UrlParts::parse("https://x.test/users/:id/:kind/:missing?id=:id");
        let mut disabled = Variable::text("kind", "admin");
        disabled.disabled = Some(true);
        parts.variable = Some(vec![Variable::text("id", "{{userId}}"), disabled]);
        let url = Url::Parts(Box::new(parts));
        assert_eq!(
            url.to_sendable_text(),
            "https://x.test/users/{{userId}}/:kind/:missing?id=:id"
        );
        assert_eq!(
            Url::Raw("https://x.test/:id".to_string()).to_sendable_text(),
            "https://x.test/:id"
        );
    }
}
