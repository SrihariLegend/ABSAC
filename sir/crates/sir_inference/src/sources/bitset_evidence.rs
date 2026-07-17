use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::Region;

use crate::engine::weights;
use crate::evidence::{Evidence, Polarity};
use sir_transform::representation::Representation;

/// Contribute evidence toward the BitSet representation.
///
/// For each semantic concept present in the region, emit an evidence
/// entry that supports BitSet.
///
/// This is a pure function: it reads the region, returns evidence.
/// The caller owns the registry and handles aggregation.
pub fn contribute(region: &Region, truths: &[sir_semantics::truth::SemanticTruth]) -> Vec<Evidence> {
    let mut evidence = Vec::new();

    if region.contains(SemanticConcept::LogicalSequence) || truths.iter().any(|t| t.concept == SemanticConcept::LogicalSequence && t.origin == region.id) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::STRONG,
            source: SemanticConcept::LogicalSequence,
            explanation: "Boolean arrays often represent bitsets",
        });
    }

    if region.contains(SemanticConcept::FiniteCollection) || truths.iter().any(|t| t.concept == SemanticConcept::FiniteCollection && t.origin == region.id) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::MODERATE,
            source: SemanticConcept::FiniteCollection,
            explanation: "Known iteration bound enables bitwise encoding",
        });
    }

    if region.contains(SemanticConcept::MembershipTraversal) || truths.iter().any(|t| t.concept == SemanticConcept::MembershipTraversal && t.origin == region.id) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::STRONG,
            source: SemanticConcept::MembershipTraversal,
            explanation: "Testing membership is a bitset operation",
        });
    }

    if region.contains(SemanticConcept::CardinalityReduction) || truths.iter().any(|t| t.concept == SemanticConcept::CardinalityReduction && t.origin == region.id) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::MODERATE,
            source: SemanticConcept::CardinalityReduction,
            explanation: "Counting members matches popcount pattern",
        });
    }

    if region.contains(SemanticConcept::DisjunctiveReduction) || truths.iter().any(|t| t.concept == SemanticConcept::DisjunctiveReduction && t.origin == region.id) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::MODERATE,
            source: SemanticConcept::DisjunctiveReduction,
            explanation: "Checking any members matches bitwise OR pattern",
        });
    }

    if region.contains(SemanticConcept::ConjunctiveReduction) || truths.iter().any(|t| t.concept == SemanticConcept::ConjunctiveReduction && t.origin == region.id) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::MODERATE,
            source: SemanticConcept::ConjunctiveReduction,
            explanation: "Checking all members matches bitwise AND pattern",
        });
    }

    if region.contains(SemanticConcept::ExclusiveReduction) || truths.iter().any(|t| t.concept == SemanticConcept::ExclusiveReduction && t.origin == region.id) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::MODERATE,
            source: SemanticConcept::ExclusiveReduction,
            explanation: "Checking parity matches bitwise XOR pattern",
        });
    }

    if region.contains(SemanticConcept::BitsetIteration) || truths.iter().any(|t| t.concept == SemanticConcept::BitsetIteration && t.origin == region.id) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::STRONG,
            source: SemanticConcept::BitsetIteration,
            explanation: "Looping over set bits is definitive evidence for BitSet",
        });
    }

    evidence
}
