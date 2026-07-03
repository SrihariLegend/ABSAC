use serde::{Deserialize, Serialize};
use std::fmt;

use sir_semantics::concepts::SemanticConcept;
use sir_types::RegionId;
use sir_transform::context::ContextId;
use sir_transform::ids::DefinitionId;

/// Unique identifier for a candidate plan.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CandidateId(pub u64);

impl CandidateId {
    pub fn new(id: u64) -> Self { Self(id) }
}

impl fmt::Display for CandidateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "candidate#{}", self.0)
    }
}

/// How a bitset transformation might be implemented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImplementationStrategy {
    /// Iterate over set bits: while bb != 0 { tzcnt; bb &= bb-1 }
    BitIteration,
    /// Compute cardinality directly: popcount(bb)
    Popcount,
    /// Change data representation: bool[64] → u64
    PackedBitfield,
    /// Replace boolean predicates with mask operations: AND/OR/XOR
    MaskConstruction,
}

impl fmt::Display for ImplementationStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImplementationStrategy::BitIteration => write!(f, "BitIteration"),
            ImplementationStrategy::Popcount => write!(f, "Popcount"),
            ImplementationStrategy::PackedBitfield => write!(f, "PackedBitfield"),
            ImplementationStrategy::MaskConstruction => write!(f, "MaskConstruction"),
        }
    }
}

/// What kind of change a candidate proposes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateEffect {
    /// The representation of data changes (e.g., bool[64] → u64)
    RepresentationChange,
    /// How the data is traversed changes (e.g., loop → trailing-zero scan)
    TraversalChange,
    /// How predicates test conditions changes (e.g., if → mask)
    PredicateEncodingChange,
    /// How counting is performed changes (e.g., accumulator → popcount)
    CountingStrategyChange,
}

/// Human-readable explanation of a candidate plan.
#[derive(Clone, Debug)]
pub struct CandidateExplanation {
    pub source_concepts: Vec<SemanticConcept>,
    pub rationale: &'static str,
}

/// A candidate transformation plan — a proposed implementation strategy
/// for a region, derived from a TransformationContext.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub id: CandidateId,
    pub region: RegionId,
    /// Reference to the context that produced this candidate.
    /// Multiple candidates may reference the same context.
    pub context_id: ContextId,
    pub definition_id: DefinitionId,
    pub strategy: ImplementationStrategy,
    pub explanation: CandidateExplanation,
    pub effects: Vec<CandidateEffect>,
    /// Expected cost profile after this transformation is applied.
    /// Set by the generator at creation time based on the implementation strategy.
    /// This is objective data — cost models assign meaning to it.
    pub expected_cost: sir_types::CostProfile,
}
