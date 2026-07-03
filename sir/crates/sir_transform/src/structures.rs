use serde::{Deserialize, Serialize};

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
