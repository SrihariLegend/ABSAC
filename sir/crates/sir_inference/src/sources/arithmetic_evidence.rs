use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::Region;

use crate::engine::weights;
use crate::evidence::{Evidence, Polarity};
use sir_transform::representation::Representation;

/// Contribute evidence toward the BitwiseArithmetic representation.
pub fn contribute(region: &Region) -> Vec<Evidence> {
    let mut evidence = Vec::new();

    if region.contains(SemanticConcept::ModuloPowerOfTwo) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitwiseArithmetic,
            polarity: Polarity::Supports,
            weight: weights::ABSOLUTE,
            source: SemanticConcept::ModuloPowerOfTwo,
            explanation: "Modulo by a power of two is exactly equivalent to bitwise AND",
        });
    }

    if region.contains(SemanticConcept::MultiplyPowerOfTwo) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitwiseArithmetic,
            polarity: Polarity::Supports,
            weight: weights::ABSOLUTE,
            source: SemanticConcept::MultiplyPowerOfTwo,
            explanation: "Multiplication by a power of two is exactly equivalent to left shift",
        });
    }

    evidence
}
