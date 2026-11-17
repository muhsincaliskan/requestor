use crate::collection::{Param, Variable};
use crate::text::{Description, StringOrList};

const FILE_KIND: &str = "file";
const TEXT_KIND: &str = "text";

#[derive(Debug, Clone, PartialEq)]
pub struct KeyValueRow {
    pub enabled: bool,
    pub key: String,
    pub value: String,
    pub description: String,
    pub is_file: bool,
    source: Option<Param>,
}

impl Default for KeyValueRow {
    fn default() -> Self {
        KeyValueRow {
            enabled: true,
            key: String::new(),
            value: String::new(),
            description: String::new(),
            is_file: false,
            source: None,
        }
    }
}

impl KeyValueRow {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        KeyValueRow {
            key: key.into(),
            value: value.into(),
            ..KeyValueRow::default()
        }
    }

    pub fn from_param(param: &Param) -> Self {
        let is_file = param.kind.as_deref() == Some(FILE_KIND);
        let value = match (&param.src, is_file) {
            (Some(source), true) => source.joined(","),
            _ => param.value.clone().unwrap_or_default(),
        };
        KeyValueRow {
            enabled: param.is_enabled(),
            key: param.key.clone().unwrap_or_default(),
            value,
            description: description_text(param.description.as_ref()),
            is_file,
            source: Some(param.clone()),
        }
    }

    pub fn from_variable(variable: &Variable) -> Self {
        KeyValueRow {
            enabled: variable.is_enabled(),
            key: variable.key.clone().unwrap_or_default(),
            value: variable.text_value().unwrap_or_default(),
            description: description_text(variable.description.as_ref()),
            ..KeyValueRow::default()
        }
    }

    pub fn from_query_pair(pair: &str) -> Self {
        KeyValueRow::from_param(&Param::from_query_pair(pair))
    }

    pub fn to_param(&self) -> Param {
        let mut param = self.source.clone().unwrap_or_else(|| Param::text("", ""));
        param.key = Some(self.key.clone());
        param.disabled = (!self.enabled).then_some(true);
        param.description = self.merged_description(param.description.take());
        if self.is_file {
            param.kind = Some(FILE_KIND.to_string());
            param.src = Some(StringOrList::Single(self.value.clone()));
            param.value = None;
        } else {
            if param.kind.as_deref() == Some(FILE_KIND) {
                param.kind = Some(TEXT_KIND.to_string());
            }
            param.src = None;
            param.value = self.has_value().then(|| self.value.clone());
        }
        param
    }

    pub fn to_variable(&self) -> Variable {
        Variable {
            disabled: (!self.enabled).then_some(true),
            description: self.merged_description(None),
            ..Variable::text(&self.key, &self.value)
        }
    }

    pub fn query_pair(&self) -> String {
        if self.has_value() {
            format!("{}={}", self.key, self.value)
        } else {
            self.key.clone()
        }
    }

    pub fn is_blank(&self) -> bool {
        self.key.is_empty() && self.value.is_empty() && self.description.is_empty()
    }

    pub fn is_active(&self) -> bool {
        self.enabled && !self.is_blank()
    }

    pub fn active_count(rows: &[KeyValueRow]) -> usize {
        rows.iter().filter(|row| row.is_active()).count()
    }

    pub fn commit_into(self, rows: &mut Vec<KeyValueRow>) -> Option<KeyValueRow> {
        if self.is_blank() {
            return Some(self);
        }
        rows.push(self);
        None
    }

    fn has_value(&self) -> bool {
        let valueless_source = self
            .source
            .as_ref()
            .is_some_and(|param| param.value.is_none());
        !(self.value.is_empty() && valueless_source)
    }

    fn merged_description(&self, existing: Option<Description>) -> Option<Description> {
        if description_text(existing.as_ref()) == self.description {
            return existing;
        }
        match existing {
            Some(Description::Rich {
                kind,
                version,
                extra,
                ..
            }) => Some(Description::Rich {
                content: Some(self.description.clone()),
                kind,
                version,
                extra,
            }),
            _ if self.description.is_empty() => None,
            _ => Some(Description::Text(self.description.clone())),
        }
    }
}

pub fn rows_from_params(params: Option<&Vec<Param>>) -> Vec<KeyValueRow> {
    params
        .into_iter()
        .flatten()
        .map(KeyValueRow::from_param)
        .collect()
}

pub fn params_from_rows(rows: &[KeyValueRow]) -> Vec<Param> {
    rows.iter()
        .filter(|row| !row.is_blank())
        .map(KeyValueRow::to_param)
        .collect()
}

fn description_text(description: Option<&Description>) -> String {
    description
        .and_then(Description::content)
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::KeyValueRow;
    use crate::collection::Param;

    #[test]
    fn unknown_param_fields_survive_an_edit() {
        let param: Param =
            serde_json::from_str(r#"{"key":"a","value":"1","x-custom":true,"description":"d"}"#)
                .unwrap();
        let mut row = KeyValueRow::from_param(&param);
        row.value = "2".to_string();
        row.enabled = false;
        let edited = row.to_param();
        assert_eq!(edited.value.as_deref(), Some("2"));
        assert_eq!(edited.disabled, Some(true));
        assert!(edited.extra.contains_key("x-custom"));
        assert_eq!(edited.description, param.description);
    }

    #[test]
    fn file_rows_keep_the_path_in_src() {
        let row = KeyValueRow::from_param(&Param::file("doc", "/tmp/a.pdf"));
        assert!(row.is_file);
        assert_eq!(row.value, "/tmp/a.pdf");
        let param = row.to_param();
        assert_eq!(param.kind.as_deref(), Some("file"));
        assert!(param.value.is_none());
    }

    #[test]
    fn turning_a_file_row_into_text_drops_the_source_path() {
        let mut row = KeyValueRow::from_param(&Param::file("doc", "/tmp/a.pdf"));
        row.is_file = false;
        row.value = "hello".to_string();
        let param = row.to_param();
        assert_eq!(param.kind.as_deref(), Some("text"));
        assert!(param.src.is_none());
        assert_eq!(param.value.as_deref(), Some("hello"));
    }

    #[test]
    fn valueless_query_pairs_stay_valueless() {
        assert_eq!(
            KeyValueRow::from_param(&Param::from_query_pair("flag")).query_pair(),
            "flag"
        );
        assert_eq!(KeyValueRow::new("a", "").query_pair(), "a=");
        assert_eq!(KeyValueRow::new("a", "1").query_pair(), "a=1");
    }

    #[test]
    fn a_typed_row_joins_the_table_and_a_blank_one_stays_pending() {
        let mut rows = vec![KeyValueRow::new("a", "1")];
        let pending = KeyValueRow::default().commit_into(&mut rows);
        assert!(pending.is_some());
        assert_eq!(rows.len(), 1);
        assert!(KeyValueRow::new("b", "").commit_into(&mut rows).is_none());
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn active_count_skips_disabled_and_blank_rows() {
        let mut disabled = KeyValueRow::new("off", "1");
        disabled.enabled = false;
        let rows = vec![KeyValueRow::new("a", "1"), disabled, KeyValueRow::default()];
        assert_eq!(KeyValueRow::active_count(&rows), 1);
    }
}
