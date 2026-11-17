use super::row::{rows_from_params, KeyValueRow};
use super::RequestDraft;
use crate::collection::{path_variable_names, Url};

const QUERY_MARK: char = '?';
const HASH_MARK: char = '#';
const QUERY_SEPARATOR: &str = "&";

struct UrlSections<'a> {
    address: &'a str,
    query: Option<&'a str>,
    fragment: &'a str,
}

impl<'a> UrlSections<'a> {
    fn of(url: &'a str) -> Self {
        let (before_hash, fragment) = match url.find(HASH_MARK) {
            Some(position) => url.split_at(position),
            None => (url, ""),
        };
        let (address, query) = match before_hash.split_once(QUERY_MARK) {
            Some((address, query)) => (address, Some(query)),
            None => (before_hash, None),
        };
        UrlSections {
            address,
            query,
            fragment,
        }
    }
}

impl RequestDraft {
    pub fn url_edited(&mut self) {
        let parsed = self.query_rows_from_url();
        let disabled = self.query.iter().filter(|row| !row.enabled).cloned();
        self.query = parsed.into_iter().chain(disabled).collect();
        self.sync_path_variables();
    }

    pub fn query_edited(&mut self) {
        let sections = UrlSections::of(&self.url);
        let pairs: Vec<String> = self
            .query
            .iter()
            .filter(|row| row.enabled && !row.is_blank())
            .map(KeyValueRow::query_pair)
            .collect();
        let query = if pairs.is_empty() {
            String::new()
        } else {
            format!("{QUERY_MARK}{}", pairs.join(QUERY_SEPARATOR))
        };
        self.url = format!("{}{query}{}", sections.address, sections.fragment);
    }

    pub(super) fn load_url_tables(&mut self, url: Option<&Url>) {
        match url {
            Some(Url::Parts(parts)) => {
                self.query = match &parts.query {
                    Some(query) => rows_from_params(Some(query)),
                    None => self.query_rows_from_url(),
                };
                self.path_variables = parts
                    .variable
                    .iter()
                    .flatten()
                    .map(KeyValueRow::from_variable)
                    .collect();
            }
            _ => self.query = self.query_rows_from_url(),
        }
        self.sync_path_variables();
    }

    fn query_rows_from_url(&self) -> Vec<KeyValueRow> {
        let mut previous = self.query.iter().filter(|row| row.enabled);
        UrlSections::of(&self.url)
            .query
            .into_iter()
            .flat_map(|query| query.split(QUERY_SEPARATOR))
            .filter(|pair| !pair.is_empty())
            .map(|pair| {
                let parsed = KeyValueRow::from_query_pair(pair);
                match previous.next().filter(|old| old.key == parsed.key) {
                    Some(old) => old.refreshed_by(parsed),
                    None => parsed,
                }
            })
            .collect()
    }

    fn sync_path_variables(&mut self) {
        let names = path_variable_names(&self.url);
        let mut rows: Vec<KeyValueRow> = Vec::with_capacity(names.len());
        for name in names {
            if rows.iter().any(|row| row.key == name) {
                continue;
            }
            let existing = self.path_variables.iter().find(|row| row.key == name);
            rows.push(
                existing
                    .cloned()
                    .unwrap_or_else(|| KeyValueRow::new(name, "")),
            );
        }
        self.path_variables = rows;
    }
}
