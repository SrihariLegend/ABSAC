use sir_inference::evidence::{Evidence, Polarity};
use sir_transform::representation::Representation;
use sir_inference::hypothesis::{Hypothesis, Support};
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::RegionId;

#[test]
fn support_score_is_positive_minus_negative() {
    let s = Support { positive: 85, negative: 15 };
    assert_eq!(s.score(), 70);
}

#[test]
fn support_ratio_is_positive_over_total() {
    let s = Support { positive: 75, negative: 25 };
    assert!((s.ratio() - 0.75).abs() < 0.001);
}

#[test]
fn support_ratio_zero_total() {
    let s = Support { positive: 0, negative: 0 };
    assert_eq!(s.ratio(), 0.0);
}

#[test]
fn support_confidence_labels() {
    assert_eq!(Support { positive: 10, negative: 0 }.confidence_label(), "Weak");
    assert_eq!(Support { positive: 30, negative: 0 }.confidence_label(), "Moderate");
    assert_eq!(Support { positive: 55, negative: 0 }.confidence_label(), "Strong");
    assert_eq!(Support { positive: 80, negative: 0 }.confidence_label(), "Strong"); // net 80 = Strong
    assert_eq!(Support { positive: 0, negative: 81 }.confidence_label(), "Very Strong"); // net 81 = Very Strong
    assert_eq!(Support { positive: 85, negative: 0 }.confidence_label(), "Very Strong");
}

#[test]
fn evidence_supports_bit_set_from_boolean_collection() {
    let evidence = Evidence {
        region: RegionId::new(0),
        representation: Representation::BitSet,
        polarity: Polarity::Supports,
        weight: 30,
        source: SemanticConcept::BooleanCollection,
        explanation: "Boolean arrays often represent bitsets",
    };
    assert_eq!(evidence.representation, Representation::BitSet);
    assert!(matches!(evidence.polarity, Polarity::Supports));
}

#[test]
fn evidence_against_has_negative_effect() {
    let evidence = Evidence {
        region: RegionId::new(0),
        representation: Representation::BitSet,
        polarity: Polarity::Against,
        weight: 30,
        source: SemanticConcept::MembershipTraversal,
        explanation: "Mutation argues against immutable bitset",
    };
    assert!(matches!(evidence.polarity, Polarity::Against));
}

#[test]
fn hypothesis_stores_representation_with_support_and_evidence() {
    let h = Hypothesis {
        representation: Representation::BitSet,
        support: Support { positive: 80, negative: 10 },
        evidence: vec![0, 1, 2],
    };
    assert_eq!(h.representation, Representation::BitSet);
    assert_eq!(h.support.score(), 70);
}

#[test]
fn representation_display() {
    assert_eq!(format!("{}", Representation::BitSet), "BitSet");
}
