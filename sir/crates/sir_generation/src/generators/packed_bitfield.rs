use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;

use crate::candidate::{
    Candidate, CandidateEffects, CandidateExplanation, CandidateId,
    ImplementationStrategy,
};

/// Plan a PackedBitfield candidate: replaces bool[64] with a single u64.
///
/// strategy: replace the array-of-booleans representation with a packed integer
pub fn plan(context: &TransformationContext) -> Option<Candidate> {
    if context.representation != Representation::BitSet {
        return None;
    }

    let candidate = Candidate {
        id: CandidateId::new(0),
        region: context.region,
        context_id: sir_transform::context::ContextId::new(0),
        strategy: ImplementationStrategy::PackedBitfield,
        explanation: CandidateExplanation {
            strategy: ImplementationStrategy::PackedBitfield,
            representation: Representation::BitSet,
            source_concepts: vec![
                SemanticConcept::BooleanCollection,
                SemanticConcept::FiniteCollection,
            ],
            prerequisites: context.constraints.iter().cloned().collect(),
            rationale: "Replace the bool[64] array with a single u64 value, \
                        reducing memory footprint from 64 bytes to 8 bytes \
                        and enabling bitwise operations on the entire set.",
        },
        effects: vec![
            CandidateEffects::RepresentationChange,
        ],
    };

    Some(candidate)
}
