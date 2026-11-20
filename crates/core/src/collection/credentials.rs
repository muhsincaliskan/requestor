use serde_json::Value;

use super::{Auth, AuthAttribute, Collection, Item, Param, Request, RequestSpec, Variable};
use crate::secret::{is_literal, is_secret_name};

const AUTH_SECRET_KEYS: [&str; 15] = [
    "authKey",
    "secret",
    "privateKey",
    "clientToken",
    "accessKey",
    "token",
    "password",
    "value",
    "accessToken",
    "refreshToken",
    "clientSecret",
    "consumerSecret",
    "tokenSecret",
    "secretKey",
    "sessionToken",
];

impl AuthAttribute {
    fn holds_literal_secret(&self) -> bool {
        let secret_key = self
            .key
            .as_deref()
            .is_some_and(|key| AUTH_SECRET_KEYS.contains(&key));
        secret_key
            && self
                .value
                .as_ref()
                .and_then(Value::as_str)
                .is_some_and(is_literal)
    }
}

impl Auth {
    pub fn literal_secret_count(&self) -> usize {
        self.attributes()
            .filter(|attribute| attribute.holds_literal_secret())
            .count()
    }

    pub fn clear_literal_secrets(&mut self) {
        self.parameters
            .values_mut()
            .flatten()
            .flatten()
            .filter(|attribute| attribute.holds_literal_secret())
            .for_each(|attribute| attribute.set_text(""));
    }

    fn attributes(&self) -> impl Iterator<Item = &AuthAttribute> {
        self.parameters.values().flatten().flatten()
    }
}

impl Variable {
    fn holds_unmarked_secret(&self) -> bool {
        !self.is_secret() && is_secret_name(self.key_text()) && is_literal(&self.value_text())
    }
}

fn unmarked_secret_count(variables: Option<&Vec<Variable>>) -> usize {
    variables
        .into_iter()
        .flatten()
        .filter(|variable| variable.holds_unmarked_secret())
        .count()
}

fn clear_unmarked_secrets(variables: Option<&mut Vec<Variable>>) {
    variables
        .into_iter()
        .flatten()
        .filter(|variable| variable.holds_unmarked_secret())
        .for_each(|variable| variable.set_value_text(""));
}

impl Param {
    fn holds_literal_secret(&self) -> bool {
        self.has_secret_name() && self.value.as_deref().is_some_and(is_literal)
    }
}

impl Request {
    fn literal_secret_count(&self) -> usize {
        let headers = self
            .header
            .iter()
            .flatten()
            .filter(|header| header.holds_literal_secret())
            .count();
        headers + self.auth.as_ref().map_or(0, Auth::literal_secret_count)
    }

    fn clear_literal_secrets(&mut self) {
        self.header
            .iter_mut()
            .flatten()
            .filter(|header| header.holds_literal_secret())
            .for_each(|header| header.value = Some(String::new()));
        if let Some(auth) = self.auth.as_mut() {
            auth.clear_literal_secrets();
        }
    }
}

impl Item {
    fn literal_secret_count(&self) -> usize {
        let own_auth = self.auth.as_ref().map_or(0, Auth::literal_secret_count);
        let request = match &self.request {
            Some(RequestSpec::Full(request)) => request.literal_secret_count(),
            _ => 0,
        };
        let nested: usize = self.children().iter().map(Item::literal_secret_count).sum();
        let examples: usize = self
            .example_requests()
            .map(|request| request.literal_secret_count())
            .sum();
        own_auth + request + nested + examples + unmarked_secret_count(self.variable.as_ref())
    }

    fn clear_literal_secrets(&mut self) {
        if let Some(auth) = self.auth.as_mut() {
            auth.clear_literal_secrets();
        }
        if let Some(RequestSpec::Full(request)) = self.request.as_mut() {
            request.clear_literal_secrets();
        }
        clear_unmarked_secrets(self.variable.as_mut());
        self.clear_example_secrets();
        self.item
            .iter_mut()
            .flatten()
            .for_each(Item::clear_literal_secrets);
    }
}

impl Item {
    fn example_requests(&self) -> impl Iterator<Item = Request> + '_ {
        (0..self.response.as_ref().map_or(0, Vec::len))
            .filter_map(|index| self.example(index)?.request())
    }

    fn clear_example_secrets(&mut self) {
        let count = self.response.as_ref().map_or(0, Vec::len);
        for index in 0..count {
            let Some(mut example) = self.example(index) else {
                continue;
            };
            let Some(RequestSpec::Full(request)) = example.original_request.as_mut() else {
                continue;
            };
            if request.literal_secret_count() == 0 {
                continue;
            }
            request.clear_literal_secrets();
            if let (Some(slot), Ok(value)) = (
                self.response
                    .as_mut()
                    .and_then(|examples| examples.get_mut(index)),
                serde_json::to_value(&example),
            ) {
                *slot = value;
            }
        }
    }
}

impl Collection {
    pub fn literal_secret_count(&self) -> usize {
        let own = self.auth.as_ref().map_or(0, Auth::literal_secret_count)
            + unmarked_secret_count(self.variable.as_ref());
        own + self
            .item
            .iter()
            .map(Item::literal_secret_count)
            .sum::<usize>()
    }

    pub fn shared(&self, keep_credentials: bool) -> Collection {
        if keep_credentials {
            self.clone()
        } else {
            self.without_literal_secrets()
        }
    }

    pub fn without_literal_secrets(&self) -> Collection {
        let mut stripped = self.clone();
        if let Some(auth) = stripped.auth.as_mut() {
            auth.clear_literal_secrets();
        }
        clear_unmarked_secrets(stripped.variable.as_mut());
        stripped
            .item
            .iter_mut()
            .for_each(Item::clear_literal_secrets);
        stripped
    }
}
