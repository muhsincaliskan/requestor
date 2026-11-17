const PAIR_SEPARATOR: char = '=';
const ATTRIBUTE_SEPARATOR: char = ';';

pub struct CookieEntry<'a> {
    pub name: &'a str,
    pub value: &'a str,
    pub attributes: &'a str,
}

impl<'a> CookieEntry<'a> {
    pub fn parse(raw: &'a str) -> Self {
        let (pair, attributes) = raw.split_once(ATTRIBUTE_SEPARATOR).unwrap_or((raw, ""));
        let (name, value) = pair.split_once(PAIR_SEPARATOR).unwrap_or((pair, ""));
        CookieEntry {
            name: name.trim(),
            value: value.trim(),
            attributes: attributes.trim(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CookieEntry;

    #[test]
    fn splits_the_pair_from_its_attributes() {
        let cookie = CookieEntry::parse("sid=abc=1; Path=/; HttpOnly");
        assert_eq!(cookie.name, "sid");
        assert_eq!(cookie.value, "abc=1");
        assert_eq!(cookie.attributes, "Path=/; HttpOnly");
    }

    #[test]
    fn tolerates_missing_value_and_attributes() {
        let bare = CookieEntry::parse("flag");
        assert_eq!((bare.name, bare.value, bare.attributes), ("flag", "", ""));
        let empty = CookieEntry::parse("");
        assert_eq!((empty.name, empty.value), ("", ""));
    }
}
