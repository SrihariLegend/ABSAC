use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use sir_types::NodeId;

/// Describes the physical organization of data in a region.
///
/// SourceStructure describes data layout, not computational behavior.
/// Computational behavior belongs to the semantic layer (SemanticConcept).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceStructure {
    /// Array of booleans with known length, e.g. bool[64]
    BooleanArray { length: usize },
    /// Single integer used as a bitmask, e.g. u64 storing flags
    BitMask { width: usize },
    /// Multiple boolean values packed into minimal storage
    PackedBooleanArray { element_count: usize },
    /// 2D arrangement of boolean values
    BooleanMatrix { rows: usize, cols: usize },
}

impl SourceStructure {
    /// Return the set of SIR nodes that constitute this structure (v0.1 stub).
    pub fn nodes(&self) -> Option<BTreeSet<NodeId>> {
        None // v0.1: structural regions are identified by roles, not node enumeration
    }
}
