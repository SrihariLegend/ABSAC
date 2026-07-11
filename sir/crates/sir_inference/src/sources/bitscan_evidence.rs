use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::Region;

use crate::engine::weights;
use crate::evidence::{Evidence, Polarity};
use sir_transform::representation::Representation;

/// Contribute evidence toward the BitScan representation.
pub fn contribute(region: &Region) -> Vec<Evidence> {
    let mut evidence = Vec::new();

    if region.contains(SemanticConcept::FirstOccurrence) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitScan,
            polarity: Polarity::Supports,
            weight: weights::ABSOLUTE,
            source: SemanticConcept::FirstOccurrence,
            explanation: "Searching for the first true element is equivalent to BitScanForward",
        });
    }

    if region.contains(SemanticConcept::LastOccurrence) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitScan,
            polarity: Polarity::Supports,
            weight: weights::ABSOLUTE,
            source: SemanticConcept::LastOccurrence,
            explanation: "Searching for the last true element is equivalent to BitScanReverse",
        });
    }

    if region.contains(SemanticConcept::TrailingZeroSearch) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitScan,
            polarity: Polarity::Supports,
            weight: weights::ABSOLUTE,
            source: SemanticConcept::TrailingZeroSearch,
            explanation: "Counting trailing zeros maps directly to hardware TrailingZeroCount",
        });
    }

    if region.contains(SemanticConcept::LeadingZeroSearch) {
        evidence.push(Evidence {
            region: region.id,
            representation: Representation::BitScan,
            polarity: Polarity::Supports,
            weight: weights::ABSOLUTE,
            source: SemanticConcept::LeadingZeroSearch,
            explanation: "Counting leading zeros maps directly to hardware LeadingZeroCount",
        });
    }

    evidence
}
