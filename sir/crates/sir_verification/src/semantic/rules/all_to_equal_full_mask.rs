use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;

/// Rewrite: All(BooleanArray(v)) → EqualFullMask(Pack(BooleanArray(v)))
///
/// This is the mathematical identity that powers the BS003 proof.
#[derive(Clone, Debug)]
pub struct AllToEqualFullMask;

impl NormalizationRule for AllToEqualFullMask {
    fn name(&self) -> &'static str {
        "AllToEqualFullMask"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        match expr {
            SemanticExpression::All(inner) => match inner.as_ref() {
                SemanticExpression::LogicalSequence { variable } => {
                    Some(SemanticExpression::EqualFullMask(Box::new(
                        SemanticExpression::Pack(Box::new(SemanticExpression::LogicalSequence {
                            variable: *variable,
                        })),
                    )))
                }
                _ => None,
            },
            _ => None,
        }
    }
}
