use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;

use crate::candidate::{
    Candidate, CandidateEffects, CandidateExplanation, CandidateId,
    ImplementationStrategy,
};

/// Data-driven strategy definition for a bitset transformation plan.
struct StrategyDef {
    strategy: ImplementationStrategy,
    source_concepts: &'static [SemanticConcept],
    rationale: &'static str,
    effects: &'static [CandidateEffects],
}

impl StrategyDef {
    fn build(&self, context: &TransformationContext) -> Candidate {
        Candidate {
            id: CandidateId::new(0),       // assigned by database
            region: context.region,
            context_id: context.context_id,
            strategy: self.strategy,
            explanation: CandidateExplanation {
                source_concepts: self.source_concepts.to_vec(),
                rationale: self.rationale,
            },
            effects: self.effects.to_vec(),
        }
    }
}

static STRATEGIES: &[StrategyDef] = &[
    StrategyDef {
        strategy: ImplementationStrategy::BitIteration,
        source_concepts: &[
            SemanticConcept::MembershipTraversal,
            SemanticConcept::BooleanCollection,
        ],
        rationale: "Iterate over only set bits using trailing-zero count and bit clear, \
                    visiting only populated elements rather than all 64 positions.",
        effects: &[CandidateEffects::TraversalChange],
    },
    StrategyDef {
        strategy: ImplementationStrategy::Popcount,
        source_concepts: &[
            SemanticConcept::CardinalityReduction,
            SemanticConcept::BooleanCollection,
        ],
        rationale: "Count set bits directly using hardware popcount instruction, \
                    eliminating the counting loop entirely.",
        effects: &[CandidateEffects::CountingStrategyChange],
    },
    StrategyDef {
        strategy: ImplementationStrategy::PackedBitfield,
        source_concepts: &[
            SemanticConcept::BooleanCollection,
            SemanticConcept::FiniteCollection,
        ],
        rationale: "Replace the bool[64] array with a single u64 value, \
                    reducing memory footprint from 64 bytes to 8 bytes \
                    and enabling bitwise operations on the entire set.",
        effects: &[CandidateEffects::RepresentationChange],
    },
    StrategyDef {
        strategy: ImplementationStrategy::MaskConstruction,
        source_concepts: &[
            SemanticConcept::BooleanCollection,
            SemanticConcept::MembershipTraversal,
        ],
        rationale: "Replace boolean predicate evaluation with bitmask construction, \
                    enabling parallel evaluation of multiple conditions via AND/OR/XOR.",
        effects: &[CandidateEffects::PredicateEncodingChange],
    },
];

/// Generate candidate plans for a BitSet transformation context.
///
/// Returns all applicable strategies when the context targets BitSet
/// representation. Returns an empty vec for any other representation.
pub fn all_bitset_plans(context: &TransformationContext) -> Vec<Candidate> {
    if context.representation != Representation::BitSet {
        return vec![];
    }
    STRATEGIES.iter().map(|s| s.build(context)).collect()
}
