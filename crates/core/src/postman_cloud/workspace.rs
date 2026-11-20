#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudWorkspace {
    pub id: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudItem {
    pub uid: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceContents {
    pub collections: Vec<CloudItem>,
    pub environments: Vec<CloudItem>,
}

impl WorkspaceContents {
    pub fn item_count(&self) -> usize {
        self.collections.len() + self.environments.len()
    }
}
