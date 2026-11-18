use serde_json::{Map, Value};

use super::{Auth, Collection, Item, Param, Request};

const SHARED_HEADERS_KEY: &str = "requestorHeaders";

#[derive(Debug, Clone, PartialEq)]
pub struct InheritedHeader {
    pub header: Param,
    pub owner: String,
}

impl Item {
    pub fn shared_headers(&self) -> Vec<Param> {
        read(&self.extra)
    }

    pub fn set_shared_headers(&mut self, headers: &[Param]) {
        write(&mut self.extra, headers);
    }
}

impl Collection {
    pub fn shared_headers(&self) -> Vec<Param> {
        read(&self.extra)
    }

    pub fn set_shared_headers(&mut self, headers: &[Param]) {
        write(&mut self.extra, headers);
    }

    pub fn inherited_headers(&self, id: &str) -> Vec<InheritedHeader> {
        let collection = owned(self.shared_headers(), &self.info.name);
        let folders = self
            .ancestors_of(id)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|folder| owned(folder.shared_headers(), folder.display_name()));
        collection.chain(folders).collect()
    }

    pub fn auth_owner(&self, id: &str) -> Option<(&Auth, &str)> {
        let trail = self.ancestors_of(id)?;
        trail
            .iter()
            .rev()
            .find_map(|folder| folder.auth.as_ref().zip(Some(folder.display_name())))
            .or_else(|| self.auth.as_ref().zip(Some(self.info.name.as_str())))
    }
}

impl InheritedHeader {
    pub fn effective<'a>(inherited: &'a [InheritedHeader], own_names: &[&str]) -> Vec<&'a Self> {
        pick(inherited, |entry| &entry.header, own_names)
    }
}

impl Request {
    pub fn add_inherited_headers(&mut self, inherited: &[Param]) {
        let own = self.header.get_or_insert_with(Vec::new);
        let own_names: Vec<&str> = own
            .iter()
            .filter(|header| header.is_enabled())
            .filter_map(|header| header.key.as_deref())
            .collect();
        let added: Vec<Param> = pick(inherited, |header| header, &own_names)
            .into_iter()
            .cloned()
            .collect();
        own.splice(0..0, added);
        if own.is_empty() {
            self.header = None;
        }
    }
}

fn pick<'a, T>(
    inherited: &'a [T],
    header: impl Fn(&T) -> &Param,
    own_names: &[&str],
) -> Vec<&'a T> {
    let mut picked: Vec<&T> = Vec::new();
    let mut names: Vec<&str> = own_names.to_vec();
    for entry in inherited.iter().rev() {
        let param = header(entry);
        let name = param.key.as_deref().unwrap_or_default();
        let taken = names.iter().any(|known| known.eq_ignore_ascii_case(name));
        if param.is_enabled() && !name.trim().is_empty() && !taken {
            names.push(name);
            picked.push(entry);
        }
    }
    picked.reverse();
    picked
}

fn owned(headers: Vec<Param>, owner: &str) -> impl Iterator<Item = InheritedHeader> + '_ {
    headers.into_iter().map(move |header| InheritedHeader {
        header,
        owner: owner.to_string(),
    })
}

fn read(extra: &Map<String, Value>) -> Vec<Param> {
    extra
        .get(SHARED_HEADERS_KEY)
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

fn write(extra: &mut Map<String, Value>, headers: &[Param]) {
    match serde_json::to_value(headers) {
        Ok(value) if !headers.is_empty() => extra.insert(SHARED_HEADERS_KEY.to_string(), value),
        _ => extra.remove(SHARED_HEADERS_KEY),
    };
}
