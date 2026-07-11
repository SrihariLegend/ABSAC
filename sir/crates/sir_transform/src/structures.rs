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
    LogicalSequence { length: usize },
    /// Dynamically generated boolean sequence with known length, e.g. array > scalar
    DynamicBooleanSequence { length: usize },
    /// Single integer used as a bitmask, e.g. u64 storing flags
    BitMask { width: usize },
    /// Multiple boolean values packed into minimal storage
    PackedLogicalSequence { element_count: usize },
    /// 2D arrangement of boolean values
    BooleanMatrix { rows: usize, cols: usize },
    /// Arithmetic modulo operator with a constant power-of-two divisor
    ModuloOperator,
    /// Arithmetic divide operator with a constant power-of-two divisor
    DivideOperator,
    /// Arithmetic multiply operator with a constant power-of-two multiplier
    MultiplyOperator,
    /// Shift operators that extract a mask
    ShiftMaskOperator,
}

impl SourceStructure {
    /// Return the set of SIR nodes that constitute this structure (v0.1 stub).
    pub fn nodes(&self) -> Option<BTreeSet<NodeId>> {
        None // v0.1: structural regions are identified by roles, not node enumeration
    }
}
