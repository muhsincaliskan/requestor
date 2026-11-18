use std::cmp::Ordering;

use super::JsonNode;

pub type NodePath = Vec<usize>;

impl JsonNode {
    pub fn at_path(&self, path: &[usize]) -> Option<&JsonNode> {
        path.iter().try_fold(self, |node, position| {
            node.child(*position).map(|(_, child)| child)
        })
    }

    pub fn descendants(&self) -> Vec<(NodePath, &JsonNode)> {
        let mut visited = Vec::new();
        let mut pending = vec![(NodePath::new(), self)];
        while let Some((path, node)) = pending.pop() {
            let children = (0..node.child_count()).rev().filter_map(|position| {
                let (_, child) = node.child(position)?;
                Some((extended(&path, position), child))
            });
            pending.extend(children);
            visited.push((path, node));
        }
        visited
    }

    pub fn compare(&self, other: &JsonNode) -> Option<Ordering> {
        if let (Some(left), Some(right)) = (self.as_number(), other.as_number()) {
            return left.partial_cmp(&right);
        }
        if let (Some(left), Some(right)) = (self.as_text(), other.as_text()) {
            return Some(left.cmp(right));
        }
        (self == other).then_some(Ordering::Equal)
    }
}

pub(crate) fn extended(path: &[usize], position: usize) -> NodePath {
    let mut longer = Vec::with_capacity(path.len() + 1);
    longer.extend_from_slice(path);
    longer.push(position);
    longer
}
