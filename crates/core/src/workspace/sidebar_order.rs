use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::collection::{Collection, Item};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidebarSort {
    NameAscending,
    NameDescending,
    #[default]
    #[serde(other)]
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SidebarOrder {
    pub sort: SidebarSort,
    pub folders_first: bool,
}

impl SidebarSort {
    pub const ALL: [SidebarSort; 3] = [
        SidebarSort::Manual,
        SidebarSort::NameAscending,
        SidebarSort::NameDescending,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SidebarSort::Manual => "Manual (drag to reorder)",
            SidebarSort::NameAscending => "Name A → Z",
            SidebarSort::NameDescending => "Name Z → A",
        }
    }

    fn compare(self, left: &str, right: &str) -> Ordering {
        let by_name = || left.to_lowercase().cmp(&right.to_lowercase());
        match self {
            SidebarSort::Manual => Ordering::Equal,
            SidebarSort::NameAscending => by_name(),
            SidebarSort::NameDescending => by_name().reverse(),
        }
    }
}

impl SidebarOrder {
    pub fn allows_dragging(&self) -> bool {
        self.sort == SidebarSort::Manual
    }

    pub fn is_manual(&self) -> bool {
        *self == SidebarOrder::default()
    }

    pub fn arrange_collections<'a>(
        &self,
        collections: &'a [Collection],
    ) -> Vec<(usize, &'a Collection)> {
        let mut arranged: Vec<(usize, &Collection)> = collections.iter().enumerate().collect();
        arranged
            .sort_by(|(_, left), (_, right)| self.sort.compare(&left.info.name, &right.info.name));
        arranged
    }

    pub fn arrange_items<'a>(&self, items: &'a [Item]) -> Vec<&'a Item> {
        let mut arranged: Vec<&Item> = items.iter().collect();
        arranged.sort_by(|left, right| self.compare_items(left, right));
        arranged
    }

    fn compare_items(&self, left: &Item, right: &Item) -> Ordering {
        let folders = if self.folders_first {
            right.is_folder().cmp(&left.is_folder())
        } else {
            Ordering::Equal
        };
        folders.then_with(|| self.sort.compare(left.display_name(), right.display_name()))
    }
}
