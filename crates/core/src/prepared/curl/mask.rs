use crate::collection::SCHEME_SEPARATOR;
use crate::header::Header;
use crate::media_type;
use crate::secret::{is_secret_name, REDACTED};

const QUERY_MARK: char = '?';
const PAIR_SEPARATOR: &str = "&";
const VALUE_SEPARATOR: char = '=';
const PATH_START: char = '/';
const USERINFO_END: char = '@';
const PASSWORD_START: char = ':';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SecretExposure {
    Masked,
    Included,
}

impl SecretExposure {
    pub(super) fn masks(self) -> bool {
        self == SecretExposure::Masked
    }

    pub(super) fn header_value(self, header: &Header) -> String {
        if self.masks() && header.is_sensitive() {
            REDACTED.to_string()
        } else {
            header.value.clone()
        }
    }

    pub(super) fn url(self, url: &str) -> String {
        if !self.masks() {
            return url.to_string();
        }
        match url.split_once(QUERY_MARK) {
            Some((base, query)) => {
                format!("{}{QUERY_MARK}{}", mask_userinfo(base), mask_pairs(query))
            }
            None => mask_userinfo(url),
        }
    }

    pub(super) fn inline_body(self, body: &str, content_type: Option<&str>) -> String {
        let is_form =
            content_type.is_some_and(|kind| kind.starts_with(media_type::FORM_URLENCODED));
        if self.masks() && is_form {
            mask_pairs(body)
        } else {
            body.to_string()
        }
    }
}

fn mask_pairs(text: &str) -> String {
    text.split(PAIR_SEPARATOR)
        .map(mask_pair)
        .collect::<Vec<_>>()
        .join(PAIR_SEPARATOR)
}

fn mask_pair(pair: &str) -> String {
    match pair.split_once(VALUE_SEPARATOR) {
        Some((key, _)) if is_secret_name(key) => format!("{key}{VALUE_SEPARATOR}{REDACTED}"),
        _ => pair.to_string(),
    }
}

fn mask_userinfo(base: &str) -> String {
    let Some((scheme, rest)) = base.split_once(SCHEME_SEPARATOR) else {
        return base.to_string();
    };
    let (authority, path) = rest.split_at(rest.find(PATH_START).unwrap_or(rest.len()));
    match authority.rsplit_once(USERINFO_END) {
        Some((userinfo, host)) if userinfo.contains(PASSWORD_START) => {
            let user = userinfo.split(PASSWORD_START).next().unwrap_or_default();
            format!("{scheme}{SCHEME_SEPARATOR}{user}{PASSWORD_START}{REDACTED}{USERINFO_END}{host}{path}")
        }
        _ => base.to_string(),
    }
}
