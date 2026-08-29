use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::Region;
use sir_transform::representation::Representation;

use crate::engine::weights;
use crate::evidence::{Evidence, Polarity};

/// Contribute evidence for the `BitPermutation` representation.
///
/// Looks for permutation concepts that describe a wholesale rearrangement
/// of bit positions (circular rotations and composed swap pipelines).
/// The physical shift/mask stages are never the representation's subject —
/// only the derived permutation concepts are.
pub fn contribute(region: &Region) -> Vec<Evidence> {
    let mut evidence = Vec::new();

    if region.contains(SemanticConcept::CircularPermutation) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitPermutation,
            polarity: Polarity::Supports,
            weight: weights::ABSOLUTE,
            source: SemanticConcept::CircularPermutation,
            explanation: "A circular rotation permutes bit positions around a word",
        });
    }

    if region.contains(SemanticConcept::BytePermutation) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitPermutation,
            polarity: Polarity::Supports,
            weight: weights::ABSOLUTE,
            source: SemanticConcept::BytePermutation,
            explanation: "Reversing byte order permutes groups of bit positions",
        });
    }

    if region.contains(SemanticConcept::BitPermutation) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitPermutation,
            polarity: Polarity::Supports,
            weight: weights::ABSOLUTE,
            source: SemanticConcept::BitPermutation,
            explanation: "Reversing bit order permutes individual bit positions",
        });
    }

    evidence
}
