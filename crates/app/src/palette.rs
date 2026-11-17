use requestor_core::workspace::ThemeName;

use crate::shortcuts::{self, Shortcut};
use crate::state::{Action, AppState};

const ENVIRONMENT_PREFIX: &str = "Use environment: ";
const NO_ENVIRONMENT_TITLE: &str = "Use no environment";
const THEME_PREFIX: &str = "Theme: ";

pub enum Target {
    Command(&'static Shortcut),
    OpenItem {
        collection_id: String,
        item_id: String,
    },
    Environment(Option<String>),
    Theme(ThemeName),
}

pub struct Entry {
    pub title: String,
    pub detail: String,
    pub method: Option<String>,
    target: Target,
}

pub struct Query {
    words: Vec<String>,
}

impl Entry {
    pub fn all(state: &AppState) -> Vec<Entry> {
        let mut entries: Vec<Entry> = shortcuts::ALL.iter().map(Entry::command).collect();
        entries.extend(Entry::requests(state));
        entries.extend(Entry::environments(state));
        entries.extend(ThemeName::ALL.into_iter().map(Entry::theme));
        entries
    }

    pub fn shortcut(&self) -> Option<&'static Shortcut> {
        match self.target {
            Target::Command(shortcut) => Some(shortcut),
            _ => None,
        }
    }

    pub fn run(&self, state: &mut AppState, actions: &mut Vec<Action>) {
        match &self.target {
            Target::Command(shortcut) => shortcut.run(state, actions),
            Target::OpenItem {
                collection_id,
                item_id,
            } => actions.push(Action::OpenItem {
                collection_id: collection_id.clone(),
                item_id: item_id.clone(),
            }),
            Target::Environment(id) => actions.push(Action::SelectEnvironment(id.clone())),
            Target::Theme(theme) => {
                state.workspace.settings.interface.theme = *theme;
                actions.push(Action::SettingsEdited);
            }
        }
    }

    fn command(shortcut: &'static Shortcut) -> Entry {
        Entry::plain(shortcut.label.to_string(), Target::Command(shortcut))
    }

    fn requests(state: &AppState) -> impl Iterator<Item = Entry> + '_ {
        state.workspace.collections.iter().flat_map(|collection| {
            collection
                .request_locations()
                .into_iter()
                .filter_map(move |location| {
                    let item_id = location.item.id.clone()?;
                    Some(Entry {
                        title: location.item.display_name().to_string(),
                        detail: location.folders,
                        method: location.item.method_name().map(str::to_string),
                        target: Target::OpenItem {
                            collection_id: collection.id().to_string(),
                            item_id,
                        },
                    })
                })
        })
    }

    fn environments(state: &AppState) -> impl Iterator<Item = Entry> + '_ {
        let none = Entry::plain(NO_ENVIRONMENT_TITLE.to_string(), Target::Environment(None));
        let named = state.workspace.environments.iter().map(|environment| {
            Entry::plain(
                format!("{ENVIRONMENT_PREFIX}{}", environment.name),
                Target::Environment(environment.id.clone()),
            )
        });
        std::iter::once(none).chain(named)
    }

    fn theme(theme: ThemeName) -> Entry {
        Entry::plain(
            format!("{THEME_PREFIX}{}", theme.label()),
            Target::Theme(theme),
        )
    }

    fn plain(title: String, target: Target) -> Entry {
        Entry {
            title,
            detail: String::new(),
            method: None,
            target,
        }
    }
}

impl Query {
    pub fn new(text: &str) -> Self {
        Query {
            words: text.split_whitespace().map(str::to_lowercase).collect(),
        }
    }

    pub fn rank<'a>(&self, entries: &'a [Entry]) -> Vec<&'a Entry> {
        let mut scored: Vec<(usize, &Entry)> = entries
            .iter()
            .filter_map(|entry| Some((self.score(entry)?, entry)))
            .collect();
        scored.sort_by_key(|(score, _)| *score);
        scored.into_iter().map(|(_, entry)| entry).collect()
    }

    fn score(&self, entry: &Entry) -> Option<usize> {
        let title = entry.title.to_lowercase();
        let detail = entry.detail.to_lowercase();
        self.words.iter().try_fold(0, |total, word| {
            let in_title = title.find(word.as_str());
            let in_detail = detail.find(word.as_str()).map(|at| title.len() + at);
            Some(total + in_title.or(in_detail)?)
        })
    }
}

#[cfg(test)]
mod tests;
