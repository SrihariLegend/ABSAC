use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;

use crate::candidate::{
    Candidate, CandidateEffects, CandidateExplanation, CandidateId,
    ImplementationStrategy,
};

/// Plan a Popcount candidate: replaces loop-based counting with
/// a single popcount instruction.
///
/// strategy: popcount(bb)
pub fn plan(context: &TransformationContext) -> Option<Candidate> {
    if context.representation != Representation::BitSet {
        return None;
    }

    let candidate = Candidate {
        id: CandidateId::new(0),
        region: context.region,
        context_id: sir_transform::context::ContextId::new(0),
        strategy: ImplementationStrategy::Popcount,
        explanation: CandidateExplanation {
            strategy: ImplementationStrategy::Popcount,
            representation: Representation::BitSet,
            source_concepts: vec![
                SemanticConcept::CardinalityReduction,
                SemanticConcept::BooleanCollection,
            ],
            prerequisites: context.constraints.iter().cloned().collect(),
            rationale: "Count set bits directly using hardware popcount instruction, \
                        eliminating the counting loop entirely.",
        },
        effects: vec![
            CandidateEffects::CountingStrategyChange,
        ],
    };

    Some(candidate)
}
