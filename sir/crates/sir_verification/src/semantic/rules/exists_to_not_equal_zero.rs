use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;

/// Rewrite: Exists(BooleanArray(v)) → NotEqualZero(Pack(BooleanArray(v)))
///
/// This is the mathematical identity that powers the BS002 proof.
#[derive(Clone, Debug)]
pub struct ExistsToNotEqualZero;

impl NormalizationRule for ExistsToNotEqualZero {
    fn name(&self) -> &'static str {
        "ExistsToNotEqualZero"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        match expr {
            SemanticExpression::Exists(inner) => match inner.as_ref() {
                SemanticExpression::BooleanArray { variable } => {
                    Some(SemanticExpression::NotEqualZero(Box::new(
                        SemanticExpression::Pack(Box::new(SemanticExpression::BooleanArray {
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
