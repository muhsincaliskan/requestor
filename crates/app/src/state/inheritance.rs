use requestor_core::collection::{Collection, InheritedHeader};
use requestor_core::draft::AuthDraft;

use super::{AppState, ItemBinding, TabId};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Inheritance {
    pub auth: Option<InheritedAuth>,
    pub headers: Vec<InheritedHeader>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InheritedAuth {
    pub kind: &'static str,
    pub owner: String,
}

impl Inheritance {
    fn of(collection: &Collection, item_id: &str) -> Self {
        let auth = collection
            .auth_owner(item_id)
            .map(|(auth, owner)| InheritedAuth {
                kind: AuthDraft::from_auth(Some(auth)).kind.label(),
                owner: owner.to_string(),
            });
        Inheritance {
            auth,
            headers: collection.inherited_headers(item_id),
        }
    }
}

impl AppState {
    pub fn inheritance(&self, tab: TabId) -> Inheritance {
        self.tab(tab)
            .and_then(|open| open.scope())
            .and_then(|scope: &ItemBinding| {
                let collection = self.workspace.collection(&scope.collection_id)?;
                Some(Inheritance::of(collection, &scope.item_id))
            })
            .unwrap_or_default()
    }
}
