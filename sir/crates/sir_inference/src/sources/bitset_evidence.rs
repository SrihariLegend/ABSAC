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
pub fn contribute(region: &Region) -> Vec<Evidence> {
    let mut evidence = Vec::new();

    if region.contains(SemanticConcept::BooleanCollection) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::STRONG,
            source: SemanticConcept::BooleanCollection,
            explanation: "Boolean arrays often represent bitsets",
        });
    }

    if region.contains(SemanticConcept::FiniteCollection) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::MODERATE,
            source: SemanticConcept::FiniteCollection,
            explanation: "Known iteration bound enables bitwise encoding",
        });
    }

    if region.contains(SemanticConcept::MembershipTraversal) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::STRONG,
            source: SemanticConcept::MembershipTraversal,
            explanation: "Testing membership is a bitset operation",
        });
    }

    if region.contains(SemanticConcept::CardinalityReduction) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitSet,
            polarity: Polarity::Supports,
            weight: weights::MODERATE,
            source: SemanticConcept::CardinalityReduction,
            explanation: "Counting members matches popcount pattern",
        });
    }

    evidence
}
