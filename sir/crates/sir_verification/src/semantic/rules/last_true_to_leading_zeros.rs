use crate::semantic::expression::SemanticExpression;
use crate::semantic::normalizer::NormalizationRule;

pub struct LastTrueToLeadingZeros;

impl NormalizationRule for LastTrueToLeadingZeros {
    fn name(&self) -> &'static str {
        "LastTrueToLeadingZeros"
    }

    fn apply(&self, expr: &SemanticExpression) -> Option<SemanticExpression> {
        if let SemanticExpression::LastTrue(inner) = expr {
            return Some(SemanticExpression::LeadingZeros(Box::new(
                SemanticExpression::Pack(inner.clone()),
            )));
        }
        None
    }
}
