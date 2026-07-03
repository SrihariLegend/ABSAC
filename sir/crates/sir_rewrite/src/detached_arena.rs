use std::collections::btree_map::{self, BTreeMap};
use std::iter::Map;

use sir_nodes::Node;

use crate::local_id::LocalNodeId;

/// A self-contained arena for replacement SIR nodes.
///
/// Identical in structure to `NodeArena` but keyed by `LocalNodeId`
/// instead of `NodeId`. Holds the replacement subgraph before it is
/// spliced into the cloned function by `RewriteBuilder`.
///
/// Uses `BTreeMap` for deterministic iteration order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DetachedArena {
    nodes: BTreeMap<LocalNodeId, Node>,
}

impl DetachedArena {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
        }
    }

    /// Insert a node. The node's `id` field (NodeId) is ignored during
    /// detached construction — the `LocalNodeId` key is the authoritative
    /// identifier. Returns the old node if the LocalNodeId was already used.
    pub fn insert(&mut self, local_id: LocalNodeId, node: Node) -> Option<Node> {
        self.nodes.insert(local_id, node)
    }

    /// Get a reference to a node by its local ID.
    pub fn get(&self, id: LocalNodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Check whether a local ID exists in the arena.
    pub fn contains(&self, id: LocalNodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Return the number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Return true if empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Iterate over all nodes in sorted order (by LocalNodeId).
    pub fn iter(&self) -> impl Iterator<Item = (LocalNodeId, &Node)> {
        self.nodes.iter().map(|(id, node)| (*id, node))
    }

    /// Iterate over node references in sorted order.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    /// Return a reference to the underlying BTreeMap.
    pub fn inner(&self) -> &BTreeMap<LocalNodeId, Node> {
        &self.nodes
    }
}

type IterMap<'a> = Map<
    btree_map::Iter<'a, LocalNodeId, Node>,
    fn((&'a LocalNodeId, &'a Node)) -> (LocalNodeId, &'a Node),
>;

impl<'a> IntoIterator for &'a DetachedArena {
    type Item = (LocalNodeId, &'a Node);
    type IntoIter = IterMap<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.iter().map(|(id, node)| (*id, node))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_nodes::NodeKind;
    use sir_types::{ConstantData, Effects, Span, Type};

    fn make_node(id: u64, val: i32) -> Node {
        sir_nodes::Node::new(
            sir_types::NodeId::new(id),
            NodeKind::Constant(ConstantData::i32(val)),
            Type::i32(),
            Effects::empty(),
            Span::unknown(),
        )
    }

    #[test]
    fn insert_and_get() {
        let mut arena = DetachedArena::new();
        assert!(arena.is_empty());

        let node = make_node(0, 42);
        let local = LocalNodeId::new(0);
        assert!(arena.insert(local, node.clone()).is_none());
        assert_eq!(arena.len(), 1);

        let retrieved = arena.get(local).unwrap();
        match &retrieved.kind {
            NodeKind::Constant(data) => assert_eq!(*data, ConstantData::i32(42)),
            _ => panic!("expected Constant"),
        }
    }

    #[test]
    fn insert_duplicate_rejects() {
        let mut arena = DetachedArena::new();
        let local = LocalNodeId::new(1);
        arena.insert(local, make_node(0, 1));
        let rejected = arena.insert(local, make_node(0, 2));
        assert!(rejected.is_some());
    }

    #[test]
    fn iteration_is_sorted() {
        let mut arena = DetachedArena::new();
        arena.insert(LocalNodeId::new(3), make_node(3, 30));
        arena.insert(LocalNodeId::new(1), make_node(1, 10));
        arena.insert(LocalNodeId::new(2), make_node(2, 20));

        let ids: Vec<u64> = arena.iter().map(|(id, _)| id.as_u64()).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }
}
