use std::fmt::Write;

const ITEM_TEMPLATE_BYTES_ESTIMATE: usize = 150;

pub fn json_of_size(target_bytes: usize) -> String {
    let mut text = String::with_capacity(target_bytes + ITEM_TEMPLATE_BYTES_ESTIMATE);
    text.push_str("[\n");
    let mut index = 0usize;
    while text.len() < target_bytes {
        if index > 0 {
            text.push_str(",\n");
        }
        write_item(&mut text, index);
        index += 1;
    }
    text.push_str("\n]\n");
    text
}

fn write_item(text: &mut String, index: usize) {
    let _ = write!(
        text,
        "  {{\n    \"id\": {index},\n    \"name\": \"item-{index}\",\n    \"active\": {},\n    \"score\": {}.5,\n    \"tags\": [\"alpha\", \"beta\"]\n  }}",
        index.is_multiple_of(2),
        index % 97,
    );
}

#[cfg(test)]
mod tests {
    use super::json_of_size;

    #[test]
    fn generates_valid_json_of_at_least_the_requested_size() {
        let text = json_of_size(10_000);
        assert!(text.len() >= 10_000);
        assert!(serde_json::from_str::<serde_json::Value>(&text).is_ok());
    }
}
