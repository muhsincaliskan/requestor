use super::KeyValueRow;

const DISABLED_MARK: &str = "//";
const SEPARATOR: char = ':';
const LINE_BREAK: &str = "\n";

impl KeyValueRow {
    pub fn bulk_text(rows: &[KeyValueRow]) -> String {
        rows.iter()
            .filter(|row| !row.is_blank())
            .map(KeyValueRow::bulk_line)
            .collect::<Vec<_>>()
            .join(LINE_BREAK)
    }

    pub fn rows_from_bulk(text: &str, previous: &[KeyValueRow]) -> Vec<KeyValueRow> {
        let mut unused: Vec<Option<&KeyValueRow>> = previous.iter().map(Some).collect();
        text.split(LINE_BREAK)
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| KeyValueRow::from_bulk_line(line).reusing(&mut unused))
            .collect()
    }

    fn bulk_line(&self) -> String {
        let mark = if self.enabled { "" } else { DISABLED_MARK };
        format!("{mark}{}{SEPARATOR}{}", self.key, self.value)
    }

    fn from_bulk_line(line: &str) -> KeyValueRow {
        let (enabled, rest) = match line.strip_prefix(DISABLED_MARK) {
            Some(rest) => (false, rest),
            None => (true, line),
        };
        let (key, value) = rest.split_once(SEPARATOR).unwrap_or((rest, ""));
        let mut row = KeyValueRow::new(key.trim(), value.trim());
        row.enabled = enabled;
        row
    }

    fn reusing(self, unused: &mut [Option<&KeyValueRow>]) -> KeyValueRow {
        let found = unused
            .iter_mut()
            .find(|slot| slot.is_some_and(|row| row.key == self.key))
            .and_then(Option::take);
        match found {
            Some(earlier) => {
                let mut row = earlier.clone();
                row.enabled = self.enabled;
                row.value = self.value;
                row
            }
            None => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::KeyValueRow;

    #[test]
    fn rows_turn_into_lines_and_back() {
        let mut off = KeyValueRow::new("X-Old", "1");
        off.enabled = false;
        let mut described = KeyValueRow::new("Accept", "text/html");
        described.description = "kept".to_string();
        let rows = vec![described, off, KeyValueRow::default()];
        let text = KeyValueRow::bulk_text(&rows);
        assert_eq!(text, "Accept:text/html\n//X-Old:1");
        let edited = KeyValueRow::rows_from_bulk(
            "Accept: application/json\n\n//X-Old:1\nAuthorization: Bearer a:b\nNoValue",
            &rows,
        );
        let pairs: Vec<(bool, &str, &str)> = edited
            .iter()
            .map(|row| (row.enabled, row.key.as_str(), row.value.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                (true, "Accept", "application/json"),
                (false, "X-Old", "1"),
                (true, "Authorization", "Bearer a:b"),
                (true, "NoValue", ""),
            ]
        );
        assert_eq!(edited[0].description, "kept");
    }
}
