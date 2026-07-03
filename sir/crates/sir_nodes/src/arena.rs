use serde::{Deserialize, Serialize};
use std::collections::btree_map::{self, BTreeMap};

use sir_types::NodeId;

use crate::Node;

/// An arena-based store for SIR nodes.
///
/// `NodeArena` wraps a `BTreeMap<NodeId, Node>`, providing deterministic
/// iteration order (sorted by `NodeId`), O(log n) lookup, and automatic
/// SSA uniqueness checking (duplicate `NodeId` keys are rejected on insert).
///
/// # Examples
///
/// ```ignore
/// let mut arena = NodeArena::new();
/// let node = Node::new(NodeId::new(0), ...);
/// arena.insert(node);  // returns None (success)
/// let duplicate = arena.insert(node);  // returns Some(old) — SSA violation
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NodeArena {
    nodes: BTreeMap<NodeId, Node>,
}

impl NodeArena {
    /// Create an empty arena.
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
        }
    }

    /// Insert a node into the arena.
    ///
    /// Returns `None` on success. Returns `Some(old_node)` if a node with the
    /// same `NodeId` already exists (SSA violation).
    pub fn insert(&mut self, node: Node) -> Option<Node> {
        self.nodes.insert(node.id, node)
    }

    /// Get a reference to a node by ID.
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Get a mutable reference to a node by ID.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    /// Remove a node from the arena, returning it.
    pub fn remove(&mut self, id: NodeId) -> Option<Node> {
        self.nodes.remove(&id)
    }

    /// Check whether a node with the given ID exists in the arena.
    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Return the number of nodes in the arena.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Return true if the arena is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Iterate over all nodes in sorted order (by NodeId).
    pub fn iter(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    /// Return a reference to the underlying BTreeMap.
    pub fn nodes(&self) -> &BTreeMap<NodeId, Node> {
        &self.nodes
    }
}

// Allow iteration over the arena directly.
impl<'a> IntoIterator for &'a NodeArena {
    type Item = &'a Node;
    type IntoIter = btree_map::Values<'a, NodeId, Node>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeKind;
    use sir_types::{ConstantData, Effects, Span, Type};

    fn make_node(id: u64) -> Node {
        Node::new(
            NodeId::new(id),
            NodeKind::Constant(ConstantData::i32(id as i32)),
            Type::i32(),
            Effects::empty(),
            Span::unknown(),
        )
    }

    #[test]
    fn insert_and_get() {
        let mut arena = NodeArena::new();
        assert!(arena.is_empty());

        let node = make_node(0);
        assert!(arena.insert(node.clone()).is_none());
        assert_eq!(arena.len(), 1);
        assert!(!arena.is_empty());

        let retrieved = arena.get(NodeId::new(0));
        assert_eq!(retrieved, Some(&node));
    }

    #[test]
    fn insert_duplicate_rejects() {
        let mut arena = NodeArena::new();
        let node1 = make_node(1);
        let node2 = make_node(1); // Same ID, different data

        assert!(arena.insert(node1.clone()).is_none());
        let rejected = arena.insert(node2.clone());
        assert!(rejected.is_some());
        assert_eq!(rejected.unwrap().id, NodeId::new(1));
        // The first node should still be in the arena.
        assert_eq!(arena.get(NodeId::new(1)), Some(&node1));
    }

    #[test]
    fn contains() {
        let mut arena = NodeArena::new();
        arena.insert(make_node(5));
        assert!(arena.contains(NodeId::new(5)));
        assert!(!arena.contains(NodeId::new(99)));
    }

    #[test]
    fn remove() {
        let mut arena = NodeArena::new();
        arena.insert(make_node(3));
        assert_eq!(arena.len(), 1);
        let removed = arena.remove(NodeId::new(3));
        assert!(removed.is_some());
        assert!(arena.is_empty());
        assert!(!arena.contains(NodeId::new(3)));
    }

    #[test]
    fn get_mut() {
        let mut arena = NodeArena::new();
        arena.insert(make_node(7));
        {
            let node = arena.get_mut(NodeId::new(7)).unwrap();
            node.metadata.insert("modified", "true");
        }
        assert_eq!(
            arena.get(NodeId::new(7)).unwrap().metadata.get("modified"),
            Some("true")
        );
    }

    #[test]
    fn iteration_is_sorted() {
        let mut arena = NodeArena::new();
        arena.insert(make_node(3));
        arena.insert(make_node(1));
        arena.insert(make_node(2));
        let ids: Vec<u64> = arena.iter().map(|n| n.id.as_u64()).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn into_iter() {
        let mut arena = NodeArena::new();
        arena.insert(make_node(0));
        arena.insert(make_node(1));
        let count = (&arena).into_iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn serde_roundtrip() {
        let mut arena = NodeArena::new();
        arena.insert(make_node(0));
        arena.insert(make_node(1));
        let json = serde_json::to_string(&arena).unwrap();
        let parsed: NodeArena = serde_json::from_str(&json).unwrap();
        assert_eq!(arena, parsed);
    }
}
