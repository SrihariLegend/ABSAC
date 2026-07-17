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

/// A derived semantic truth with explicit inputs and outputs.
///
/// Unlike a `Region` (which represents *where* something happens), a
/// `SemanticTruth` represents *what* mathematically occurred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticTruth {
    pub id: TruthId,
    pub concept: SemanticConcept,
    pub inputs: Vec<ValueId>,
    pub outputs: Vec<ValueId>,
    pub origin: RegionId,
    pub provenance: Provenance,
}
