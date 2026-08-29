use crate::concepts::SemanticConcept;
use crate::region::RegionId;
use sir_types::NodeId;

/// An opaque identifier for a semantic value.
///
/// This isolates Semantic Closure from SIR structure. A `ValueId`
/// maps to an SSA node in the IR, but the Closure engine only sees
/// it as a mathematical variable (`X`, `Y`, `Z`), preventing graph traversals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueId(pub u64);

impl ValueId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// An opaque identifier for a semantic truth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TruthId(pub usize);

impl TruthId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

/// Traces the origin of a semantic truth to physical nodes or ancestor truths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Provenance {
    /// Atomic truth discovered directly from AST
    Physical { nodes: Vec<NodeId> },
    /// Derived truth inheriting physical anchors from its ancestors
    Derived { from_truths: Vec<TruthId> },
}

/// Structured parameters carried by a semantic truth.
///
/// The closure engine cannot walk the SIR graph, so recognizers that need
/// parameters downstream (permutation composition, recipe width alignment)
/// attach them to the truth they emit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TruthParameter {
    /// A pair of opposite shifts of the same value: `(x << k) | (x >> (w - k))`
    /// (or the mirror image), i.e. a circular rotation.
    ShiftPair {
        /// Width of the rotated word in bits.
        width: u32,
        /// Direction of the rotation.
        direction: sir_transform::roles::ShiftDirection,
    },
    /// A masked swap of two disjoint bit groups: one side shifts the masked
    /// group left, the other shifts right, and the two halves are OR-ed.
    /// Bits `i` and `i + shift` are exchanged for every set bit `i` in `mask`.
    MaskedShiftSwap {
        /// Mask selecting the lower group of each swapped pair.
        mask: u64,
        /// Distance between the swapped groups (bits).
        shift: u32,
    },
    /// A composed permutation of bit positions: result bit `i` reads source
    /// bit `mapping[i]`; bits outside `width` are zero.
    BitPermutation {
        /// Number of bits the permutation covers.
        width: u32,
        /// `mapping[i]` = source bit feeding result bit `i`.
        mapping: Vec<u32>,
    },
}

/// A derived semantic truth with explicit inputs and outputs.
///
/// Unlike a `Region` (which represents *where* something happens), a
/// `SemanticTruth` represents *what* mathematically occurred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticTruth {
    pub parameters: Vec<TruthParameter>,
    pub id: TruthId,
    pub concept: SemanticConcept,
    pub inputs: Vec<ValueId>,
    pub outputs: Vec<ValueId>,
    pub origin: RegionId,
    pub provenance: Provenance,
}
