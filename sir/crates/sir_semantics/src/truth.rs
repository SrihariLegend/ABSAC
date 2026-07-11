use crate::concepts::SemanticConcept;
use crate::region::RegionId;

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

/// A derived semantic truth with explicit inputs and outputs.
///
/// Unlike a `Region` (which represents *where* something happens), a
/// `SemanticTruth` represents *what* mathematically occurred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticTruth {
    pub concept: SemanticConcept,
    pub inputs: Vec<ValueId>,
    pub outputs: Vec<ValueId>,
    pub origin: RegionId,
}
