//! Normalizer — canonicalization engine for SemanticExpression.
//!
//! Applies normalization rules recursively (children first, then parent) with a
//! first-match restart strategy. A `max_steps` guard prevents non-termination.

use crate::semantic::expression::SemanticExpression;
use crate::ProofStep;

/// A single semantic-preserving rewrite rule.
///
/// Invariant: A rule may only inspect the subtree rooted at the supplied
/// expression. It may not depend on global context, proof obligations,
/// or external state. This keeps normalization purely equational.
///
/// v0.1: Rules are purely structural — they match on expression shape only.
/// In future phases, if a rule's validity depends on assumptions (e.g.,
/// "this rewrite is only valid when len > 0"), the `apply` signature may
/// be extended to accept `&[Assumption]`. For now, the single BS001 rule
/// is universally valid for any BooleanArray width ≤ 128.
pub trait NormalizationRule {
    /// A human-readable name for this rule (used in proof steps).
    fn name(&self) -> &'static str;

    /// Attempt to apply this rule to the given expression.
    /// Returns None if the rule does not match.
    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression>;
}

/// A canonicalization engine for SemanticExpression.
///
/// Applies normalization rules recursively until a fixed point is reached.
/// Not a rewrite engine — rewrite engines search, normalizers reduce.
pub struct Normalizer {
    rules: Vec<Box<dyn NormalizationRule>>,
    max_steps: usize,
}

impl Normalizer {
    /// Create a new normalizer with no rules.
    pub fn new(max_steps: usize) -> Self {
        Self {
            rules: Vec::new(),
            max_steps,
        }
    }

    /// Add a normalization rule. Rules are tried in registration order.
    pub fn add_rule(&mut self, rule: Box<dyn NormalizationRule>) {
        self.rules.push(rule);
    }

    /// Normalize an expression to its canonical form.
    ///
    /// Recursively normalizes children first, then attempts to apply
    /// rules at this node. Uses a first-match restart strategy:
    /// after any successful rule application, restarts from the first rule.
    ///
    /// Returns the normal form and the sequence of applied rules (proof trace).
    pub fn normalize(
        &self,
        expr: &SemanticExpression,
    ) -> (SemanticExpression, Vec<ProofStep>) {
        let mut steps = Vec::new();
        let result = self.normalize_recursive(expr, &mut steps, 0);
        (result, steps)
    }

    /// Internal recursive normalization with step counting.
    fn normalize_recursive(
        &self,
        expr: &SemanticExpression,
        steps: &mut Vec<ProofStep>,
        depth: usize,
    ) -> SemanticExpression {
        // Guard against non-termination
        if depth >= self.max_steps {
            return expr.clone();
        }

        // Step 1: Recursively normalize children first
        let with_normalized_children = self.normalize_children(expr, steps, depth);

        // Step 2: Try to apply rules at this node with restart strategy
        let mut current = with_normalized_children;
        loop {
            let mut changed = false;
            for rule in &self.rules {
                if let Some(reduced) = rule.apply(&current) {
                    steps.push(ProofStep::Normalization {
                        rule: rule.name(),
                        before: current.clone(),
                        after: reduced.clone(),
                    });
                    current = reduced;
                    changed = true;
                    break; // restart from first rule
                }
            }
            if !changed {
                break;
            }
            // Safety: prevent infinite rule cycles
            if steps.len() >= self.max_steps {
                break;
            }
        }

        current
    }

    /// Recursively normalize all children of an expression.
    fn normalize_children(
        &self,
        expr: &SemanticExpression,
        steps: &mut Vec<ProofStep>,
        depth: usize,
    ) -> SemanticExpression {
        match expr {
            // Leaf nodes — no children to normalize
            SemanticExpression::Variable(_)
            | SemanticExpression::Constant(_)
            | SemanticExpression::BooleanArray { .. } => expr.clone(),

            // Unary nodes — normalize the single child
            SemanticExpression::Pack(inner) => {
                let normalized = self.normalize_recursive(inner, steps, depth + 1);
                SemanticExpression::Pack(Box::new(normalized))
            }
            SemanticExpression::Count(inner) => {
                let normalized = self.normalize_recursive(inner, steps, depth + 1);
                SemanticExpression::Count(Box::new(normalized))
            }
            SemanticExpression::Popcount(inner) => {
                let normalized = self.normalize_recursive(inner, steps, depth + 1);
                SemanticExpression::Popcount(Box::new(normalized))
            }
            SemanticExpression::Exists(inner) => {
                let normalized = self.normalize_recursive(inner, steps, depth + 1);
                SemanticExpression::Exists(Box::new(normalized))
            }
            SemanticExpression::All(inner) => {
                let normalized = self.normalize_recursive(inner, steps, depth + 1);
                SemanticExpression::All(Box::new(normalized))
            }
            SemanticExpression::Parity(inner) => {
                let normalized = self.normalize_recursive(inner, steps, depth + 1);
                SemanticExpression::Parity(Box::new(normalized))
            }
            SemanticExpression::NotEqualZero(inner) => {
                let normalized = self.normalize_recursive(inner, steps, depth + 1);
                SemanticExpression::NotEqualZero(Box::new(normalized))
            }
            SemanticExpression::EqualFullMask(inner) => {
                let normalized = self.normalize_recursive(inner, steps, depth + 1);
                SemanticExpression::EqualFullMask(Box::new(normalized))
            }
            SemanticExpression::BitwiseAndOne(inner) => {
                let normalized = self.normalize_recursive(inner, steps, depth + 1);
                SemanticExpression::BitwiseAndOne(Box::new(normalized))
            }

            // Filter — normalize input (predicate has no children to normalize in v0.1)
            SemanticExpression::Filter { input, predicate } => {
                let normalized_input = self.normalize_recursive(input, steps, depth + 1);
                SemanticExpression::Filter {
                    input: Box::new(normalized_input),
                    predicate: predicate.clone(),
                }
            }
        }
    }
}

impl Default for Normalizer {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_transform::ids::VariableId;

    /// A test rule that rewrites Count(BooleanArray(v)) → Constant(0).
    /// Used only for testing the normalizer framework.
    struct CountToZero;

    impl NormalizationRule for CountToZero {
        fn name(&self) -> &'static str {
            "CountToZero"
        }

        fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
            match expr {
                SemanticExpression::Count(inner) => match inner.as_ref() {
                    SemanticExpression::BooleanArray { .. } => {
                        Some(SemanticExpression::Constant(sir_types::ConstantData::u64(0)))
                    }
                    _ => None,
                },
                _ => None,
            }
        }
    }

    #[test]
    fn normalizer_empty_rules_is_identity() {
        let normalizer = Normalizer::new(100);
        let expr = SemanticExpression::Count(Box::new(
            SemanticExpression::BooleanArray { variable: VariableId::new(0) },
        ));
        let (result, steps) = normalizer.normalize(&expr);
        assert_eq!(result, expr);
        assert!(steps.is_empty());
    }

    #[test]
    fn normalizer_applies_single_rule() {
        let mut normalizer = Normalizer::new(100);
        normalizer.add_rule(Box::new(CountToZero));

        let expr = SemanticExpression::Count(Box::new(
            SemanticExpression::BooleanArray { variable: VariableId::new(0) },
        ));
        let (result, steps) = normalizer.normalize(&expr);

        assert_eq!(
            result,
            SemanticExpression::Constant(sir_types::ConstantData::u64(0))
        );
        assert_eq!(steps.len(), 1);
        assert!(matches!(steps[0], ProofStep::Normalization { rule: "CountToZero", .. }));
    }

    #[test]
    fn normalizer_reaches_fixed_point() {
        // Rule: Count(BooleanArray) → Constant(0)
        // After applying, there's no Count to match — fixed point reached
        let mut normalizer = Normalizer::new(100);
        normalizer.add_rule(Box::new(CountToZero));

        let expr = SemanticExpression::Count(Box::new(
            SemanticExpression::BooleanArray { variable: VariableId::new(0) },
        ));
        let (result, steps) = normalizer.normalize(&expr);
        assert_eq!(steps.len(), 1); // applied once, no more matches

        // Normalize again — should be idempotent
        let (result2, steps2) = normalizer.normalize(&result);
        assert_eq!(result2, result);
        assert!(steps2.is_empty());
    }

    #[test]
    fn normalizer_recursively_normalizes_children() {
        // Pack(Count(BooleanArray(v))) — rule matches Count inside Pack
        let mut normalizer = Normalizer::new(100);
        normalizer.add_rule(Box::new(CountToZero));

        let expr = SemanticExpression::Pack(Box::new(
            SemanticExpression::Count(Box::new(
                SemanticExpression::BooleanArray { variable: VariableId::new(0) },
            )),
        ));
        let (result, steps) = normalizer.normalize(&expr);

        assert_eq!(
            result,
            SemanticExpression::Pack(Box::new(
                SemanticExpression::Constant(sir_types::ConstantData::u64(0))
            ))
        );
        assert_eq!(steps.len(), 1); // Count inside Pack was normalized
    }

    #[test]
    fn normalizer_respects_max_steps() {
        // A rule that loops: Count(x) → Count(x) — would infinite loop
        struct LoopingRule;

        impl NormalizationRule for LoopingRule {
            fn name(&self) -> &'static str { "Loop" }
            fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
                match expr {
                    SemanticExpression::Count(_) => Some(expr.clone()),
                    _ => None,
                }
            }
        }

        let mut normalizer = Normalizer::new(10);
        normalizer.add_rule(Box::new(LoopingRule));

        let expr = SemanticExpression::Count(Box::new(
            SemanticExpression::BooleanArray { variable: VariableId::new(0) },
        ));
        let (_, steps) = normalizer.normalize(&expr);

        // Should stop at max_steps, not loop forever
        assert!(steps.len() <= 10);
    }
}
