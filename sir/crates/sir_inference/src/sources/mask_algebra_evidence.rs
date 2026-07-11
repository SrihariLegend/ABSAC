use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::Region;
use sir_transform::representation::Representation;

use crate::engine::weights;
use crate::evidence::{Evidence, Polarity};

/// Contribute evidence for the `MaskAlgebra` representation.
///
/// Looks for bitmask manipulation concepts (e.g., ClearLowestSetBit).
pub fn contribute(region: &Region) -> Vec<Evidence> {
    let mut evidence = Vec::new();

    if region.contains(SemanticConcept::ClearLowestSetBit) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::MaskAlgebra,
            polarity: Polarity::Supports,
            weight: weights::ABSOLUTE,
            source: SemanticConcept::ClearLowestSetBit,
            explanation: "Clearing the lowest set bit is an operation in mask algebra",
        });
    }

    evidence
}
