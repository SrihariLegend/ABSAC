use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;

use crate::candidate::{
    Candidate, CandidateEffects, CandidateExplanation, CandidateId,
    ImplementationStrategy,
};

/// Plan a BitIteration candidate: replaces full iteration with
/// trailing-zero-based iteration over only set bits.
///
/// strategy: while bb != 0 { tzcnt; bb &= bb-1 }
pub fn plan(context: &TransformationContext) -> Option<Candidate> {
    if context.representation != Representation::BitSet {
        return None;
    }

    let candidate = Candidate {
        id: CandidateId::new(0), // ID assigned by database
        region: context.region,
        context_id: sir_transform::context::ContextId::new(0), // assigned by database
        strategy: ImplementationStrategy::BitIteration,
        explanation: CandidateExplanation {
            strategy: ImplementationStrategy::BitIteration,
            representation: Representation::BitSet,
            source_concepts: vec![
                SemanticConcept::MembershipTraversal,
                SemanticConcept::BooleanCollection,
            ],
            prerequisites: context.constraints.iter().cloned().collect(),
            rationale: "Iterate over only set bits using trailing-zero count and bit clear, \
                        visiting only populated elements rather than all 64 positions.",
        },
        effects: vec![
            CandidateEffects::TraversalChange,
        ],
    };

    Some(candidate)
}
