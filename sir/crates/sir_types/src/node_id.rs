use serde::{Deserialize, Serialize};

/// A globally unique identifier for a node within a module.
///
/// NodeIds are monotonically increasing within a function, never reused,
/// and serve as keys in the `NodeArena`. They are `Copy` to allow
/// cheap references throughout the IR graph.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Create a new NodeId from a raw u64 value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the raw u64 value.
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.0)
    }
}
