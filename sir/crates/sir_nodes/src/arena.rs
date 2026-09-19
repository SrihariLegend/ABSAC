use serde::{Deserialize, Serialize};

use sir_types::NodeId;

use crate::Node;

/// An arena-based store for SIR nodes.
///
/// Nodes are indexed by `NodeId` in a dense `Vec`, so lookup is O(1)
/// and iteration is in ascending ID order. `None` slots are removed or
/// never-assigned ids.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NodeArena {
    nodes: Vec<Option<Node>>,
    len: usize,
}

impl NodeArena {
    /// Create an empty arena.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            len: 0,
        }
    }

    #[inline]
    fn ensure(&mut self, id: NodeId) {
        let index = id.0 as usize;
        if index >= self.nodes.len() {
            self.nodes.resize_with(index + 1, || None);
        }
    }

    /// Insert a node into the arena.
    ///
    /// Returns `None` on success. Returns `Some(old_node)` if a node with the
    /// same `NodeId` already exists (SSA violation).
    pub fn insert(&mut self, node: Node) -> Option<Node> {
        self.ensure(node.id);
        let slot = &mut self.nodes[node.id.0 as usize];
        let old = slot.take();
        if old.is_none() {
            self.len += 1;
        }
        *slot = Some(node);
        old
    }

    /// Get a reference to a node by ID.
    #[inline]
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0 as usize).and_then(|slot| slot.as_ref())
    }

    /// Get a mutable reference to a node by ID.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes
            .get_mut(id.0 as usize)
            .and_then(|slot| slot.as_mut())
    }

    /// Remove a node from the arena, returning it.
    pub fn remove(&mut self, id: NodeId) -> Option<Node> {
        let slot = self.nodes.get_mut(id.0 as usize)?;
        let old = slot.take();
        if old.is_some() {
            self.len -= 1;
        }
        old
    }

    /// Check whether a node with the given ID exists in the arena.
    #[inline]
    pub fn contains(&self, id: NodeId) -> bool {
        self.get(id).is_some()
    }

    /// Return the number of nodes in the arena.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return true if the arena is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterate over all nodes in ascending ID order.
    pub fn iter(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter().filter_map(|slot| slot.as_ref())
    }

    /// Iterate over all node IDs in ascending order.
    pub fn ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| slot.as_ref().map(|_| NodeId::new(index as u64)))
    }

    /// Iterate over node references in ascending ID order.
    pub fn values(&self) -> impl Iterator<Item = &Node> {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a NodeArena {
    type Item = &'a Node;
    type IntoIter = std::iter::Flatten<std::slice::Iter<'a, Option<Node>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.iter().flatten()
    }
}
