mod edits;
mod rows;

use edits::{edit_script, Edit};

pub const CONTEXT_LINES: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffRow {
    Kept(String),
    Added(String),
    Removed(String),
    Folded(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LineDiff {
    pub rows: Vec<DiffRow>,
    pub added: usize,
    pub removed: usize,
}

impl LineDiff {
    pub fn between(before: &str, after: &str) -> Option<LineDiff> {
        let old: Vec<&str> = before.lines().collect();
        let new: Vec<&str> = after.lines().collect();
        let script = edit_script(&old, &new)?;
        let added = script.iter().filter(|edit| **edit == Edit::Insert).count();
        let removed = script.iter().filter(|edit| **edit == Edit::Delete).count();
        Some(LineDiff {
            rows: rows::folded(&script, &old, &new, CONTEXT_LINES),
            added,
            removed,
        })
    }

    pub fn is_unchanged(&self) -> bool {
        self.added == 0 && self.removed == 0
    }
}

#[cfg(test)]
mod tests;
