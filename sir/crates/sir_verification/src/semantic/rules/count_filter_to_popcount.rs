use crate::semantic::expression::{Predicate, SemanticExpression};
use crate::semantic::normalizer::NormalizationRule;

/// Rewrite: Count(Filter(BooleanArray(v), True)) → Popcount(Pack(BooleanArray(v)))
///
/// This is the mathematical identity that powers the BS001 proof.
/// It states that counting the true elements of a boolean array is
/// equivalent to packing the array into a bitvector and counting set bits.
///
/// The rule is universally valid for any BooleanArray width ≤ 128
/// (the maximum representable in a u128 BitVector).
#[derive(Clone, Debug)]
pub struct CountFilterToPopcount;

impl NormalizationRule for CountFilterToPopcount {
    fn name(&self) -> &'static str {
        "CountFilterToPopcount"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        // Match: Count(Filter(BooleanArray(v), True))
        match expr {
            SemanticExpression::Count(inner) => match inner.as_ref() {
                SemanticExpression::Filter { input, predicate } => {
                    if *predicate != Predicate::True {
                        return None;
                    }
                    match input.as_ref() {
                        SemanticExpression::BooleanArray { variable } => {
                            // Rewrite to: Popcount(Pack(BooleanArray(v)))
                            Some(SemanticExpression::Popcount(Box::new(
                                SemanticExpression::Pack(Box::new(
                                    SemanticExpression::BooleanArray {
                                        variable: *variable,
                                    },
                                )),
                            )))
                        }
                        _ => None,
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_transform::ids::VariableId;

    #[test]
    fn rule_matches_count_filter_true_boolean_array() {
        let expr = SemanticExpression::Count(Box::new(
            SemanticExpression::Filter {
                input: Box::new(SemanticExpression::BooleanArray {
                    variable: VariableId::new(0),
                }),
                predicate: Predicate::True,
            },
        ));

        let rule = CountFilterToPopcount;
        let result = rule.apply(&expr);

        assert!(result.is_some());
        let rewritten = result.unwrap();
        // Should be Popcount(Pack(BooleanArray(v)))
        match rewritten {
            SemanticExpression::Popcount(inner) => match inner.as_ref() {
                SemanticExpression::Pack(inner2) => match inner2.as_ref() {
                    SemanticExpression::BooleanArray { variable } => {
                        assert_eq!(*variable, VariableId::new(0));
                    }
                    _ => panic!("Inner should be BooleanArray"),
                },
                _ => panic!("Should be Pack"),
            },
            _ => panic!("Should be Popcount"),
        }
    }

    #[test]
    fn rule_does_not_match_non_count() {
        let expr = SemanticExpression::Popcount(Box::new(
            SemanticExpression::BooleanArray { variable: VariableId::new(0) },
        ));
        let rule = CountFilterToPopcount;
        assert!(rule.apply(&expr).is_none());
    }

    #[test]
    fn rule_does_not_match_count_without_filter() {
        let expr = SemanticExpression::Count(Box::new(
            SemanticExpression::BooleanArray { variable: VariableId::new(0) },
        ));
        let rule = CountFilterToPopcount;
        assert!(rule.apply(&expr).is_none());
    }

    #[test]
    fn rule_does_not_match_filter_on_non_array() {
        let expr = SemanticExpression::Count(Box::new(
            SemanticExpression::Filter {
                input: Box::new(SemanticExpression::Variable(VariableId::new(1))),
                predicate: Predicate::True,
            },
        ));
        let rule = CountFilterToPopcount;
        assert!(rule.apply(&expr).is_none());
    }

    #[test]
    fn bs001_theorem_normalizes_to_identity() {
        // Count(Filter(BooleanArray(v), True)) normalizes to Popcount(Pack(BooleanArray(v)))
        let lhs = SemanticExpression::Count(Box::new(
            SemanticExpression::Filter {
                input: Box::new(SemanticExpression::BooleanArray {
                    variable: VariableId::new(0),
                }),
                predicate: Predicate::True,
            },
        ));
        let rhs = SemanticExpression::Popcount(Box::new(
            SemanticExpression::Pack(Box::new(
                SemanticExpression::BooleanArray { variable: VariableId::new(0) },
            )),
        ));

        // Apply rule to lhs
        let rule = CountFilterToPopcount;
        let normalized_lhs = rule.apply(&lhs).unwrap();

        // After normalization, lhs should equal rhs
        assert_eq!(normalized_lhs, rhs);
    }
}
