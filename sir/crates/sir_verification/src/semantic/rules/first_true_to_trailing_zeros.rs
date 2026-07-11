use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;

pub struct FirstTrueToTrailingZeros;

impl NormalizationRule for FirstTrueToTrailingZeros {
    fn name(&self) -> &'static str {
        "FirstTrueToTrailingZeros"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        if let SemanticExpression::FirstTrue(inner) = expr {
            return Some(SemanticExpression::TrailingZeros(Box::new(
                SemanticExpression::Pack(inner.clone()),
            )));
        }
        None
    }
}
