use eframe::egui::{Response, Stroke, StrokeKind, Ui};
use requestor_core::collection::Collection;

use crate::state::Action;

const DROP_STROKE_WIDTH: f32 = 1.5;
const DROP_CORNER_RADIUS: f32 = 2.0;

pub(super) struct DragCollection {
    collection_id: String,
}

pub(super) struct DragItem {
    collection_id: String,
    item_id: String,
}

#[derive(Clone, Copy)]
pub(super) enum DropSlot<'a> {
    IntoFolder(&'a str),
    Beside(&'a str),
    CollectionRoot,
}

impl DropSlot<'_> {
    fn is_item(&self, item_id: &str) -> bool {
        matches!(self, DropSlot::IntoFolder(id) | DropSlot::Beside(id) if *id == item_id)
    }

    fn move_action(&self, collection: &Collection, item_id: &str) -> Option<Action> {
        let (parent_id, index) = match self {
            DropSlot::IntoFolder(folder) => (Some((*folder).to_string()), usize::MAX),
            DropSlot::Beside(target) => collection.position_of(target)?,
            DropSlot::CollectionRoot => (None, usize::MAX),
        };
        Some(Action::MoveItem {
            collection_id: collection.id().to_string(),
            item_id: item_id.to_string(),
            parent_id,
            index,
        })
    }
}

pub(super) fn begin(response: &Response, collection_id: &str, item_id: &str) {
    if response.drag_started() {
        response.dnd_set_drag_payload(DragItem {
            collection_id: collection_id.to_string(),
            item_id: item_id.to_string(),
        });
    }
}

pub(super) fn accept(
    ui: &Ui,
    response: &Response,
    collection: &Collection,
    slot: DropSlot,
    actions: &mut Vec<Action>,
) {
    let hovering = response
        .dnd_hover_payload::<DragItem>()
        .is_some_and(|payload| payload.collection_id == collection.id());
    if hovering {
        highlight(ui, response);
    }
    let Some(payload) = response.dnd_release_payload::<DragItem>() else {
        return;
    };
    if payload.collection_id == collection.id() && !slot.is_item(&payload.item_id) {
        actions.extend(slot.move_action(collection, &payload.item_id));
    }
}

pub(super) fn begin_collection(response: &Response, collection_id: &str) {
    if response.drag_started() {
        response.dnd_set_drag_payload(DragCollection {
            collection_id: collection_id.to_string(),
        });
    }
}

pub(super) fn accept_collection(
    ui: &Ui,
    response: &Response,
    target_index: usize,
    actions: &mut Vec<Action>,
) {
    if response.dnd_hover_payload::<DragCollection>().is_some() {
        highlight(ui, response);
    }
    if let Some(payload) = response.dnd_release_payload::<DragCollection>() {
        actions.push(Action::MoveCollection {
            collection_id: payload.collection_id.clone(),
            index: target_index,
        });
    }
}

fn highlight(ui: &Ui, response: &Response) {
    let stroke = Stroke::new(DROP_STROKE_WIDTH, ui.visuals().selection.bg_fill);
    ui.painter().rect_stroke(
        response.rect,
        DROP_CORNER_RADIUS,
        stroke,
        StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use requestor_core::collection::{Collection, Item};

    use super::DropSlot;
    use crate::state::Action;

    fn sample() -> (Collection, String, String, String) {
        let mut collection = Collection::new("Shop");
        collection.ensure_ids();
        let folder = collection
            .add_item(None, Item::folder("Orders".to_string(), Vec::new()))
            .unwrap();
        let first = collection
            .add_item(Some(&folder), Item::folder("A".to_string(), Vec::new()))
            .unwrap();
        let second = collection
            .add_item(Some(&folder), Item::folder("B".to_string(), Vec::new()))
            .unwrap();
        (collection, folder, first, second)
    }

    #[test]
    fn dropping_on_a_folder_appends_to_it() {
        let (collection, folder, first, _) = sample();
        let action = DropSlot::IntoFolder(&folder).move_action(&collection, &first);
        assert!(matches!(
            action,
            Some(Action::MoveItem { parent_id: Some(parent), index: usize::MAX, .. }) if parent == folder
        ));
    }

    #[test]
    fn dropping_beside_an_item_takes_its_parent_and_index() {
        let (collection, folder, first, second) = sample();
        let action = DropSlot::Beside(&second).move_action(&collection, &first);
        assert!(matches!(
            action,
            Some(Action::MoveItem { parent_id: Some(parent), index: 1, .. }) if parent == folder
        ));
    }

    #[test]
    fn dropping_on_the_collection_moves_to_the_root() {
        let (collection, _, first, _) = sample();
        let action = DropSlot::CollectionRoot.move_action(&collection, &first);
        assert!(matches!(
            action,
            Some(Action::MoveItem {
                parent_id: None,
                index: usize::MAX,
                ..
            })
        ));
    }

    #[test]
    fn an_item_is_never_dropped_onto_itself() {
        let (_, folder, _, second) = sample();
        assert!(DropSlot::Beside(&second).is_item(&second));
        assert!(DropSlot::IntoFolder(&folder).is_item(&folder));
        assert!(!DropSlot::CollectionRoot.is_item(&folder));
    }
}
